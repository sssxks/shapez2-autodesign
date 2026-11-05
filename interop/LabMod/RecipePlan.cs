using System;
using System.Linq;
using Newtonsoft.Json.Linq;

internal enum RecipeOperation { Source, Cut, Swap, RotateCw, RotateCcw, Stack, Paint, Crystallize }

internal sealed class RecipeNode
{
    internal int Id { get; }
    internal RecipeOperation Operation { get; }
    internal string OperationName { get; }
    internal int[] Inputs { get; }
    internal string Expected { get; }
    internal string? Color { get; }

    internal RecipeNode(JObject input, int index)
    {
        Id = JsonInput.Integer(input, "id") ?? throw new ArgumentException("Recipe node id required");
        if (Id != index) throw new ArgumentException("Recipe IDs must follow execution order");
        OperationName = JsonInput.String(input, "op");
        Operation = OperationName switch
        {
            "source" => RecipeOperation.Source,
            "cut" => RecipeOperation.Cut,
            "swap" => RecipeOperation.Swap,
            "rotate-cw" => RecipeOperation.RotateCw,
            "rotate-ccw" => RecipeOperation.RotateCcw,
            "stack" => RecipeOperation.Stack,
            "paint" => RecipeOperation.Paint,
            "crystallize" => RecipeOperation.Crystallize,
            _ => throw new ArgumentException("Unknown recipe operation: " + OperationName)
        };
        var arity = Operation == RecipeOperation.Source ? 0 :
            Operation == RecipeOperation.Swap || Operation == RecipeOperation.Stack ? 2 : 1;
        Inputs = JsonInput.Array(input, "inputs").Select(value =>
        {
            if (value.Type != JTokenType.Integer) throw new ArgumentException("Recipe input must be an integer");
            return (int)value;
        }).ToArray();
        if (Inputs.Length != arity || Inputs.Any(value => value < 0 || value >= index))
            throw new ArgumentException($"Invalid inputs for recipe node {Id}");
        Expected = JsonInput.String(input, "expected");
        Color = JsonInput.OptionalString(input, "color");
        var colored = Operation == RecipeOperation.Paint || Operation == RecipeOperation.Crystallize;
        if (colored != (Color != null) || Color != null && Color.Length != 1)
            throw new ArgumentException($"Invalid color for recipe node {Id}");
        if (Operation == RecipeOperation.Source &&
            !new[] { "CuCuCuCu", "RuRuRuRu", "SuSuSuSu", "WuWuWuWu", "P-P-P-P-" }.Contains(Expected))
            throw new ArgumentException($"Invalid source at recipe node {Id}");
    }
}

internal sealed class RecipePlan
{
    internal string Target { get; }
    internal int Root { get; }
    internal RecipeNode[] Nodes { get; }
    internal RecipePlan(JObject input)
    {
        Target = JsonInput.String(input, "target");
        Root = JsonInput.Integer(input, "root") ?? throw new ArgumentException("Recipe root required");
        Nodes = JsonInput.Array(input, "nodes")
            .Select((node, index) => new RecipeNode(JsonInput.Object(node, "node"), index)).ToArray();
        if (Root < 0 || Root >= Nodes.Length) throw new ArgumentException("Recipe root is missing");
        if (Nodes[Root].Expected != Target) throw new ArgumentException("Recipe root differs from target");
    }
}
