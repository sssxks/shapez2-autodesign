using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Game.Content.Features.Fluids;
using Newtonsoft.Json.Linq;

internal static class RecipeProbe {
 internal static object Run(JObject args,string root){
  var recipe=JObject.Parse(File.ReadAllText((string)args["recipe"]));
  File.WriteAllText(Path.Combine(root,"recipe.json"),recipe.ToString());
  var session=GameBindings.Session;var ops=GameBindings.Field<ShapeOperations>(session,"ShapeOperations");
  var registry=GameBindings.Field<ShapeRegistry>(session,"ShapeRegistry");var ids=GameBindings.Field<IShapeIdManager>(session,"ShapeIdManager");
  var fluids=GameBindings.Field<FluidRegistry>(session,"FluidRegistry");
  var colors=GameBindings.Field<Dictionary<IFluid,FluidId>>(fluids,"FluidReferenceToIdLookup").Keys.OfType<ColorFluid>().ToArray();
  var values=new List<IItem>();var trace=new List<object>();
  foreach(JObject node in recipe["nodes"]){
   var op=(string)node["op"];var expected=(string)node["expected"];var inputs=node["inputs"].Select(x=>values[(int)x]).ToArray();
   IItem a=null,b=null;bool ok=true;
   switch(op){
    case "source":
     if(!new[]{"CuCuCuCu","RuRuRuRu","SuSuSuSu","WuWuWuWu","P-P-P-P-"}.Contains(expected))throw new Exception("Invalid source");
     a=registry.GetItem(ids.Resolve(expected));break;
    case "cut":ok=ops.Cut.TryExecute(inputs[0],out a,out b);break;
    case "swap":ok=ops.SwapHalves.TryExecute(inputs[0],inputs[1],out a,out b);break;
    case "rotate-cw":ok=ops.Rotate[0].TryExecute(inputs[0],out a);break;
    case "rotate-ccw":ok=ops.Rotate[1].TryExecute(inputs[0],out a);break;
    case "stack":ok=ops.Stack.TryExecute(inputs[0],inputs[1],out a);break;
    case "paint":ok=ops.PaintTopmost.TryExecute(inputs[0],colors.Single(c=>c.Color.Code.ToString()==(string)node["color"]),out a);break;
    case "crystallize":ok=ops.Crystallize.TryExecute(inputs[0],colors.Single(c=>c.Color.Code.ToString()==(string)node["color"]),out a);break;
    default:throw new Exception("Unknown recipe operation: "+op);
   }
   var selected=a?.ToString()==expected?a:b?.ToString()==expected?b:null;
   trace.Add(new{id=(int)node["id"],op,expected,outputs=new[]{a?.ToString(),b?.ToString()},selected_port=selected==a?0:1});
   if(!ok||selected==null){var failed=new{verified=false,target=(string)recipe["target"],failed_node=(int)node["id"],trace};File.WriteAllText(Path.Combine(root,"recipe-verification.json"),JObject.FromObject(failed).ToString());return failed;}
   values.Add(selected);
  }
  var result=new{verified=values[(int)recipe["root"]].ToString()==(string)recipe["target"],target=(string)recipe["target"],steps=values.Count,trace};
  File.WriteAllText(Path.Combine(root,"recipe-verification.json"),JObject.FromObject(result).ToString());return result;
 }
}

