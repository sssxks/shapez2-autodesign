using System;
using System.Collections.Generic;
using System.Linq;
using Game.Core.Map.Simulation;
using Game.Core.Trains;
using Game.Content.Features.Fluids;
using Game.Content.Features.Signals.Channels;
using Game.Content.Features.Signals.Tick;
using Newtonsoft.Json.Linq;

internal static class SaveInspection {
 internal static object Read(){
  if(string.IsNullOrEmpty(Environment.GetEnvironmentVariable("SHAPEZ_LAB_SAVE")))throw new InvalidOperationException("Set SHAPEZ_LAB_SAVE to inspect a save copy");
  var session=LabWorld.Session;var map=LabWorld.Field<MapModel>(session,"MapModel");
  var simulator=map.Simulator;
  var channels=LabWorld.Field<SignalChannelRegistry>(session,"SignalChannelRegistry");
  var shapesRegistry=LabWorld.Field<ShapeRegistry>(session,"ShapeRegistry");
  var receivers=new List<object>();var unloaders=new List<object>();
  foreach(var localized in simulator.Simulations){
   var s=localized.Simulation;var p=localized.GetOccupiedChunk(0);
   if(s is ConstantSignalSimulation constant){var config=LabWorld.Field<ConstantSignalConfiguration>(constant,"Config");receivers.Add(new{chunk=new[]{p.x,p.y,(int)p.z},channel="constant",signal=config.Value.ToString()});}
   if(s is GlobalSignalReceiverSimulation receiver){
    var config=LabWorld.Field<IGlobalSignalReceiverConfiguration>(receiver,"Config");
    var signal=channels.TryGetReadOnlyChannel(config.ChannelId,out var channel)?channel.ReadSignal(SignalTicks.FromTicks(simulator.SimulationTime)).ToString():null;
    receivers.Add(new{chunk=new[]{p.x,p.y,(int)p.z},channel=config.ChannelId.ToString(),signal});
   }
   if(s is TrainCargoUnloaderSimulation<ShapeId> shapes)unloaders.Add(new{chunk=new[]{p.x,p.y,(int)p.z},type="shape",packages=LabWorld.Field<TrainCargoToBeltFillingContainer<ShapeId>[]>(shapes,"_FillingContainers").Select(f=>new{f.Package.Amount,item=f.Package.Item.IsValid?shapesRegistry.GetItem(f.Package.Item).ToString():null}).ToArray()});
   if(s is TrainCargoUnloaderSimulation<FluidId> fluids)unloaders.Add(new{chunk=new[]{p.x,p.y,(int)p.z},type="fluid",packages=LabWorld.Field<TrainCargoToBeltFillingContainer<FluidId>[]>(fluids,"_FillingContainers").Select(f=>JToken.FromObject(f.Package)).ToArray()});
  }
  return new{map.BuildingCount,map.IslandCount,receivers,unloaders};
 }
}
