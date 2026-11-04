using System;
using System.Collections.Generic;
using System.Linq;
using Game.Core.Coordinates;
using Game.Core.Map.Simulation;
using Game.Core.Simulation;
using Game.Core.Trains;
using Game.Content.Features.Fluids;
using Game.Content.BuildingPath.Simulation;
using Newtonsoft.Json.Linq;

internal sealed class RateProbe {
 internal readonly Dictionary<(int,int,int),string> Targets;
 internal readonly HashSet<(int,int,int)> CutIslands;
 readonly List<Action> refill=new List<Action>();
 readonly Dictionary<(int,int,int),Meter> shapes=new Dictionary<(int,int,int),Meter>();
 readonly Dictionary<(int,int,int),(ProvidingFluidContainer container,string expected)> fluids=new Dictionary<(int,int,int),(ProvidingFluidContainer,string)>();
 readonly Dictionary<(int,int,int),long> volumes=new Dictionary<(int,int,int),long>();
 internal double ExpectedBeltRate;
 internal static (int,int,int) Key(JToken p)=>((int)p[0],(int)p[1],(int)p[2]);
 internal RateProbe(JObject manifest){Targets=((JArray)manifest["targets"]).ToDictionary(t=>Key(t["tile"]),t=>(string)t["item"]);CutIslands=new HashSet<(int,int,int)>(((JArray)manifest["cut_islands"]).Select(Key));}
 sealed class Meter:IItemReceiver {
  internal long Count;readonly string expected;
  internal Meter(string expected){this.expected=expected;}
  public Steps MaxStep_S=>Steps.MaxValue/4;
  public bool CanAcceptItem(IBeltItem item)=>true;
  public void HandOverItem(IBeltItem item,Ticks remaining){if(item.ToString()!=expected)throw new Exception("Wrong shape: "+item+", expected "+expected);Count++;}
 }
 internal void Attach(ISimulator simulator,JObject manifest){
  var session=LabWorld.Session;var ids=LabWorld.Field<IShapeIdManager>(session,"ShapeIdManager");var registry=LabWorld.Field<FluidRegistry>(session,"FluidRegistry");
  var colors=LabWorld.Field<Dictionary<IFluid,FluidId>>(registry,"FluidReferenceToIdLookup").Keys.OfType<ColorFluid>().ToArray();
  var mode=LabWorld.Field<GameMode>(session,"Mode");BeltSpeed speed=mode.Buildings.ForwardBelt.ConfigAs<IConveyorConfiguration>().ConveyorSpeed;
  ExpectedBeltRate=(speed.StepsPerTick*Ticks.OneSecond).Value/(double)LaneConstants.ItemSpacing.Value;
  var sources=((JArray)manifest["sources"]).ToDictionary(t=>Key(t["chunk"]),t=>(string)t["item"]);int found=0;
  foreach(var localized in simulator.Simulations){var sim=localized.Simulation;
   var p0=localized.GetOccupiedChunk(0);
   if(sim.GetType().Name.StartsWith("TrainCargoUnloaderSimulation")&&sources.TryGetValue((p0.x,p0.y,p0.z),out var item)){
    found++;
    if(sim is TrainCargoUnloaderSimulation<ShapeId> shape){var id=ids.Resolve(item);foreach(var f in LabWorld.Field<TrainCargoToBeltFillingContainer<ShapeId>[]>(shape,"_FillingContainers"))refill.Add(()=>{if(f.Package.Amount<1000)f.LoadPackage(new CargoPackage<ShapeId>(30000,id));});}
    else if(sim is TrainCargoUnloaderSimulation<FluidId> fluid){var id=registry.GetFluidId(colors.Single(f=>f.Color.Code==item[0]));foreach(var f in LabWorld.Field<TrainCargoToBeltFillingContainer<FluidId>[]>(fluid,"_FillingContainers"))refill.Add(()=>{if(f.Package.Amount<1000)f.LoadPackage(new CargoPackage<FluidId>(30000,id));});}
   }
   if(!(localized is ILocalizedTileSimulation tile))continue;
   for(int i=0;i<tile.NumOccupiedTiles;i++){var p=tile.GetOccupiedTile(i);var key=(p.x,p.y,(int)p.z);if(!Targets.TryGetValue(key,out var expected))continue;
    if(expected.Length>1 && sim is IItemSimulation belt && belt.NumItemProviders==1){var meter=new Meter(expected);belt.GetItemProvider(0).NextLane=meter;shapes.Add(key,meter);}
    if(expected.Length==1 && sim is ISimulationComposedWithFluidPortReceiver fluid){fluids.Add(key,(fluid.FluidPortReceiverContainer,expected));volumes.Add(key,0);}
   }
  }
  if(found!=sources.Count||shapes.Count!=300||fluids.Count!=180)throw new Exception($"Endpoint mismatch: unloaders {found}/{sources.Count}, shape lanes {shapes.Count}/300, fluid lanes {fluids.Count}/180");
 }
 internal void BeforeTick(){foreach(var f in refill)f();}
 internal void AfterTick(){foreach(var pair in fluids){var c=pair.Value.container;if(c.IsEmpty)continue;if((c.Fluid as ColorFluid)?.Color.Code.ToString()!=pair.Value.expected)throw new Exception("Wrong fluid at "+pair.Key);volumes[pair.Key]+=c.Value.Units;c.Take(c.Value);}}
 internal void ResetMeters(){foreach(var m in shapes.Values)m.Count=0;foreach(var key in volumes.Keys.ToArray())volumes[key]=0;}
 internal JObject Report(int seconds){
  var s=shapes.Select(p=>new{tile=p.Key,rate=p.Value.Count/(double)seconds,ratio=p.Value.Count/(seconds*ExpectedBeltRate)}).ToArray();
  var f=volumes.Select(p=>new{tile=p.Key,liters_per_second=p.Value/(double)FluidUnit.UnitsPerLiter/seconds,ratio=p.Value/(double)FluidUnit.UnitsPerLiter/seconds/(ExpectedBeltRate*10)}).ToArray();
  return JObject.FromObject(new{window_seconds=seconds,expected_items_per_second=ExpectedBeltRate,expected_liters_per_second=ExpectedBeltRate*10,min_shape_ratio=s.Min(x=>x.ratio),min_fluid_ratio=f.Min(x=>x.ratio),shape_lanes=s,fluid_lanes=f});
 }
}
