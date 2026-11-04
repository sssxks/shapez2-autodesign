using System;
using System.IO;
using Game.Core.Coordinates;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

public sealed class CmamExperiment:IBlueprintExperiment {
 bool adapter;
 // Import policy is configured before buildings are created.
 public bool KeepBuilding(GlobalTileCoordinate p)=>!adapter||AdapterProbe.KeepBuilding(p);
 public object Run(LabWorld world,JObject args,string root){
  var seconds=(int?)args["seconds"]??1;
  if(args["production_manifest"]!=null){
   var production=new ProductionProbe(world.Simulator,JObject.Parse(File.ReadAllText((string)args["production_manifest"])),(string)args["target"]);
   world.Advance(seconds,10,()=>production.BeforeTick(world.Simulator.SimulationTime,Ticks.OneSecond/10));
   var name=(string)args["name"]??"production";
   if(name.IndexOfAny(Path.GetInvalidFileNameChars())>=0)throw new ArgumentException("Invalid case name");
   var report=production.Report();
   File.WriteAllText(Path.Combine(root,name+"-production.json"),JsonConvert.SerializeObject(report,Formatting.Indented));
   if(args["min_produced"]!=null&&production.Produced<(long)args["min_produced"])throw new Exception("Production below required count: "+production.Produced);
   return report;
  }
  var probe=new AdapterProbe(world.Simulator);bool complete=false;
  world.Advance(seconds,10,()=>{if(!complete)probe.BeforeTick();},()=>{if(!complete)complete=probe.AfterTick();});
  var flow=probe.Report();File.WriteAllText(Path.Combine(root,"adapter-flow.json"),JsonConvert.SerializeObject(flow));
  if(!complete)throw new Exception("Adapter flow did not reach every inlet: "+JsonConvert.SerializeObject(flow));
  return flow;
 }
 public CmamExperiment(){}
 public void Configure(JObject args){adapter=(bool?)args["adapter_probe"]==true;}
}
