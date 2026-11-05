using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Game.Content.Features.Fluids;
using Newtonsoft.Json.Linq;

internal static class RecipeProbe
{
    internal static object Run(JObject args, string root)
    {
        var input = JsonInput.ReadFile(JsonInput.String(args, "recipe"));
        var recipe = new RecipePlan(input);
        File.WriteAllText(Path.Combine(root, "recipe.json"), input.ToString());
        var ops = GameBindings.Operations;
        var registry = GameBindings.Shapes;
        var ids = GameBindings.ShapeIds;
        var colors = GameBindings.Colors;
        var values = new List<IItem>();
        var trace = new List<object>();
        foreach (var node in recipe.Nodes)
        {
            var op = node.OperationName;
            var expected = node.Expected;
            var inputs = node.Inputs.Select(x => values[x]).ToArray();
            IItem? a = null, b = null;
            bool ok = true;
            switch (node.Operation)
            {
                case RecipeOperation.Source:
                    a = registry.GetItem(ids.Resolve(expected));
                    break;
                case RecipeOperation.Cut:
                    ok = ops.Cut.TryExecute(inputs[0], out a, out b);
                    break;
                case RecipeOperation.Swap:
                    ok = ops.SwapHalves.TryExecute(inputs[0], inputs[1], out a, out b);
                    break;
                case RecipeOperation.RotateCw:
                    ok = ops.Rotate[0].TryExecute(inputs[0], out a);
                    break;
                case RecipeOperation.RotateCcw:
                    ok = ops.Rotate[1].TryExecute(inputs[0], out a);
                    break;
                case RecipeOperation.Stack:
                    ok = ops.Stack.TryExecute(inputs[0], inputs[1], out a);
                    break;
                case RecipeOperation.Paint:
                    ok = ops.PaintTopmost.TryExecute(inputs[0], colors.Single(c => c.Color.Code.ToString() == node.Color), out a);
                    break;
                case RecipeOperation.Crystallize:
                    ok = ops.Crystallize.TryExecute(inputs[0], colors.Single(c => c.Color.Code.ToString() == node.Color), out a);
                    break;
                default:
                    throw new Exception("Unknown recipe operation: " + op);
            }
            var selected = a?.ToString() == expected ? a : b?.ToString() == expected ? b : null;
            trace.Add(new
            {
                id = node.Id,
                op,
                expected,
                outputs = new[] { a?.ToString(), b?.ToString() },
                selected_port = selected == a ? 0 : 1
            });
            if (!ok || selected == null)
            {
                var failed = new
                {
                    verified = false,
                    target = recipe.Target,
                    failed_node = node.Id,
                    trace
                };
                File.WriteAllText(Path.Combine(root, "recipe-verification.json"), JObject.FromObject(failed).ToString());
                return failed;
            }
            values.Add(selected);
        }
        var result = new
        {
            verified = values[recipe.Root].ToString() == recipe.Target,
            target = recipe.Target,
            steps = values.Count,
            trace
        };
        File.WriteAllText(Path.Combine(root, "recipe-verification.json"), JObject.FromObject(result).ToString());
        return result;
    }
}

