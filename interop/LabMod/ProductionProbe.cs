using System;
using System.Collections.Generic;
using System.Linq;
using Game.Core.Map.Simulation;
using Game.Core.Simulation;
using Game.Content.Features.Signals;
using Newtonsoft.Json.Linq;

internal sealed class ProductionProbe {
 // Supply native unloaders, keep the complete factory, and observe its existing
 // trash events. Unlike RateProbe's demand test, no production lanes are cut.
 readonly RateProbe supply;readonly List<ConstantSignalSimulation> drivers=new List<ConstantSignalSimulation>();
 readonly Dictionary<string,long> trash=new Dictionary<string,long>();readonly Dictionary<string,long> output=new Dictionary<string,long>();
 readonly string target;
 readonly Dictionary<string,long> discardedAt=new Dictionary<string,long>();
 internal ProductionProbe(ISimulator simulator,JObject manifest,string target){
  this.target=target;supply=new RateProbe(manifest);supply.Attach(simulator,manifest,true);
  var session=LabWorld.Session;var registry=LabWorld.Field<ShapeRegistry>(session,"ShapeRegistry");var ids=LabWorld.Field<IShapeIdManager>(session,"ShapeIdManager");
  var signal=BeltItemSignal.From(registry.GetItem(ids.Resolve(target)));
  foreach(var localized in simulator.Simulations){
   // The isolated world has no operator channels. Drive the receiver's normal
   // conductor at native signal ticks, preserving all downstream control logic.
   if(localized.Simulation is GlobalSignalReceiverSimulation receiver)drivers.Add(new ConstantSignalSimulation(new ConstantSignalConfiguration{Value=signal}){OutputConductor=receiver.OutputConductor});
   // This CMAM's accepted output ends at the existing trash island (9, 4).
   if(localized.Simulation is TrashSimulation sink&&localized is ILocalizedTileSimulation tile){var p=tile.GetOccupiedTile(0);var final=p.x>=180&&p.x<200&&p.y>=80&&p.y<100;
    sink.OnLaneTrashesItem.Register((item,ticks,lane)=>{var key=item.ToString();trash[key]=trash.TryGetValue(key,out var n)?n+1:1;var location=p.x+","+p.y+","+p.z+":"+key;discardedAt[location]=discardedAt.TryGetValue(location,out var d)?d+1:1;if(final)output[key]=output.TryGetValue(key,out var m)?m+1:1;});
   }
  }
  if(drivers.Count!=1)throw new Exception("Expected one target-signal receiver, got "+drivers.Count);
 }
 internal void BeforeTick(Ticks start,Ticks delta){supply.BeforeTick();foreach(var driver in drivers)driver.Update(start,delta);}
 internal long Produced=>output.TryGetValue(target,out var n)?n:0;
 internal object Report()=>new{target,produced=Produced,output,trash=trash.OrderByDescending(p=>p.Value).Take(80).ToDictionary(p=>p.Key,p=>p.Value),discarded_at=discardedAt};
}
