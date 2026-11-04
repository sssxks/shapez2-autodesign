using System;
using System.Collections.Generic;
using System.Linq;
using Game.Core.Map.Simulation;
using Game.Core.Trains;
using Game.Content.Features.Fluids;
using Newtonsoft.Json.Linq;

public sealed class UnloaderSupply {
 readonly List<Action> refill=new List<Action>();
 static (int,int,int) Key(JToken p)=>((int)p[0],(int)p[1],(int)p[2]);
 public UnloaderSupply(ISimulator simulator,JObject manifest){
  var session=GameBindings.Session;var ids=GameBindings.Field<IShapeIdManager>(session,"ShapeIdManager");var registry=GameBindings.Field<FluidRegistry>(session,"FluidRegistry");
  var colors=GameBindings.Field<Dictionary<IFluid,FluidId>>(registry,"FluidReferenceToIdLookup").Keys.OfType<ColorFluid>().ToArray();
  var sources=((JArray)manifest["sources"]).ToDictionary(t=>Key(t["chunk"]),t=>(string)t["item"]);int found=0;
  foreach(var localized in simulator.Simulations){var sim=localized.Simulation;
   var p0=localized.GetOccupiedChunk(0);
   if(sim.GetType().Name.StartsWith("TrainCargoUnloaderSimulation")&&sources.TryGetValue((p0.x,p0.y,p0.z),out var item)){
    found++;
    if(sim is TrainCargoUnloaderSimulation<ShapeId> shape){var id=ids.Resolve(item);foreach(var f in GameBindings.Field<TrainCargoToBeltFillingContainer<ShapeId>[]>(shape,"_FillingContainers"))refill.Add(()=>{if(f.Package.Amount<1000)f.LoadPackage(new CargoPackage<ShapeId>(30000,id));});}
    else if(sim is TrainCargoUnloaderSimulation<FluidId> fluid){var id=registry.GetFluidId(colors.Single(f=>f.Color.Code==item[0]));foreach(var f in GameBindings.Field<TrainCargoToBeltFillingContainer<FluidId>[]>(fluid,"_FillingContainers"))refill.Add(()=>{if(f.Package.Amount<1000)f.LoadPackage(new CargoPackage<FluidId>(30000,id));});}
   }
  }
  if(found!=sources.Count)throw new Exception($"Unloader mismatch: {found}/{sources.Count}");
 }
 public void BeforeTick(){foreach(var f in refill)f();}
}

