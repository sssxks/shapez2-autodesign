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
 UnloaderSupply supply;
 readonly Dictionary<(int,int,int),Meter> shapes=new Dictionary<(int,int,int),Meter>();
 readonly Dictionary<(int,int,int),(ProvidingFluidContainer container,string expected)> fluids=new Dictionary<(int,int,int),(ProvidingFluidContainer,string)>();
 readonly Dictionary<(int,int,int),long> volumes=new Dictionary<(int,int,int),long>();
 internal double ExpectedBeltRate;
 internal static (int,int,int) Key(JToken p)=>((int)p[0],(int)p[1],(int)p[2]);
 internal RateProbe(JObject manifest){Targets=((JArray)manifest["targets"]).ToDictionary(t=>Key(t["tile"]),t=>(string)t["item"]);CutIslands=new HashSet<(int,int,int)>(((JArray)manifest["cut_islands"]).Select(Key));}
 sealed class Meter:IItemReceiver {
  internal long Count;readonly string expected;readonly Ticks interval;Ticks credit;
  internal Meter(string expected,double rate){this.expected=expected;interval=Ticks.FromSeconds((float)(1/rate));}
  internal void Supply(Ticks delta){credit+=delta;if(credit>interval)credit=interval;}
  public Steps MaxStep_S=>LaneConstants.ItemSpacing;
  public bool CanAcceptItem(IBeltItem item)=>credit>=interval;
  public void HandOverItem(IBeltItem item,Ticks remaining){if(item.ToString()!=expected)throw new Exception("Wrong shape: "+item+", expected "+expected);credit-=interval;Count++;}
 }
 internal void Attach(ISimulator simulator,JObject manifest){
  supply=new UnloaderSupply(simulator,manifest);
  var mode=GameBindings.Field<GameMode>(GameBindings.Session,"Mode");BeltSpeed speed=mode.Buildings.ForwardBelt.ConfigAs<IConveyorConfiguration>().ConveyorSpeed;
  ExpectedBeltRate=(speed.StepsPerTick*Ticks.OneSecond).Value/(double)LaneConstants.ItemSpacing.Value;
  foreach(var localized in simulator.Simulations){var sim=localized.Simulation;
   if(!(localized is ILocalizedTileSimulation tile))continue;
   for(int i=0;i<tile.NumOccupiedTiles;i++){var p=tile.GetOccupiedTile(i);var key=(p.x,p.y,(int)p.z);if(!Targets.TryGetValue(key,out var expected))continue;
    if(expected.Length>1 && sim is IItemSimulation belt && belt.NumItemProviders==1){var meter=new Meter(expected,ExpectedBeltRate);belt.GetItemProvider(0).NextLane=meter;shapes.Add(key,meter);}
    if(expected.Length==1 && sim is ISimulationComposedWithFluidPortReceiver fluid){fluids.Add(key,(fluid.FluidPortReceiverContainer,expected));volumes.Add(key,0);}
   }
  }
  var expectedShapes=Targets.Count(p=>p.Value.Length>1);var expectedFluids=Targets.Count-expectedShapes;
  if(shapes.Count!=expectedShapes||fluids.Count!=expectedFluids)throw new Exception($"Endpoint mismatch: shape lanes {shapes.Count}/{expectedShapes}, fluid lanes {fluids.Count}/{expectedFluids}");
 }
 internal void BeforeTick(){supply.BeforeTick();foreach(var meter in shapes.Values)meter.Supply(Ticks.OneSecond/10);}
 internal void AfterTick(){foreach(var pair in fluids){var c=pair.Value.container;if(c.IsEmpty)continue;if((c.Fluid as ColorFluid)?.Color.Code.ToString()!=pair.Value.expected)throw new Exception("Wrong fluid at "+pair.Key);volumes[pair.Key]+=c.Value.Units;c.Take(c.Value);}}
 internal void ResetMeters(){foreach(var m in shapes.Values)m.Count=0;foreach(var key in volumes.Keys.ToArray())volumes[key]=0;}
 internal JObject Report(int seconds){
  var s=shapes.Select(p=>new{tile=p.Key,rate=p.Value.Count/(double)seconds,ratio=p.Value.Count/(seconds*ExpectedBeltRate)}).ToArray();
  var f=volumes.Select(p=>new{tile=p.Key,liters_per_second=p.Value/(double)FluidUnit.UnitsPerLiter/seconds,ratio=p.Value/(double)FluidUnit.UnitsPerLiter/seconds/(ExpectedBeltRate*10)}).ToArray();
  return JObject.FromObject(new{window_seconds=seconds,expected_items_per_second=ExpectedBeltRate,expected_liters_per_second=ExpectedBeltRate*10,min_shape_ratio=s.Length==0?1:s.Min(x=>x.ratio),min_fluid_ratio=f.Length==0?1:f.Min(x=>x.ratio),shape_lanes=s,fluid_lanes=f});
 }
}
