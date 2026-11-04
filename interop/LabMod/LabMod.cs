using System;
using System.IO;
using System.Diagnostics;
using System.Linq;
using Game.Core.Content;
using Game.Content.BuildingPath.Simulation;
using Newtonsoft.Json.Linq;
using UnityEngine;

public sealed class ShapezLabMod : IMod {
 private readonly GameObject host;
 public ShapezLabMod(){
  if(string.IsNullOrEmpty(Environment.GetEnvironmentVariable("SHAPEZ_LAB_RUN")))return;
  host=new GameObject("ShapezLabHost");UnityEngine.Object.DontDestroyOnLoad(host);host.AddComponent<LabService>();
 }
 public void Dispose(){if(host!=null)UnityEngine.Object.Destroy(host);}
}
public sealed class LabService:MonoBehaviour {
 private string root;private bool ready;private LabPipe pipe;
 void Awake(){root=Environment.GetEnvironmentVariable("SHAPEZ_LAB_RUN");Application.runInBackground=true;pipe=new LabPipe(Environment.GetEnvironmentVariable("SHAPEZ_LAB_PIPE"));}
 void OnDestroy(){pipe?.Dispose();}
 void Update(){try{
  if(pipe.QuitRequested){if(pipe.Failure!=null)UnityEngine.Debug.LogException(pipe.Failure);Application.Quit();return;}
  if(!ready){
   if(LabBootstrap.Session==null)return;
   var process=Process.GetCurrentProcess();pipe.Reply(new{kind="ready",protocol=1,pid=process.Id,data_path=GameEnvironment.DataPath,version=Application.version,startup_seconds=(DateTime.Now-process.StartTime).TotalSeconds});ready=true;
  }
  if(!pipe.TryReceive(out var request))return;
  var id=(ulong)request["id"];
  try{var timer=Stopwatch.StartNew();var result=Execute((string)request["operation"],(JObject)request["args"]);pipe.Reply(new{id,ok=true,wall_seconds=timer.Elapsed.TotalSeconds,result});}
  catch(Exception e){pipe.Reply(new{id,ok=false,error=e.ToString()});}
 }catch(Exception e){UnityEngine.Debug.LogException(e);pipe?.Dispose();enabled=false;Application.Quit();}}
 private object Execute(string operation,JObject args){
  switch(operation){
   case "shutdown":return new{shutdown=true};
   case "recipe":return RecipeProbe.Run(args,root);
   case "inspect-save":return SaveInspection.Read(args);
   case "capacities":
    var mode=GameBindings.Mode;
    BeltSpeed belt=mode.Buildings.ForwardBelt.ConfigAs<IConveyorConfiguration>().ConveyorSpeed;
    BeltSpeed space=mode.Islands.SpaceBelts.First().ConfigAs<SpacePathConfiguration>().SpaceConveyorSpeed;
    BeltSpeed pipe=mode.Islands.SpacePipes.First().ConfigAs<SpacePathConfiguration>().SpaceConveyorSpeed;
    return new{belt=belt.ToString(),space=space.ToString(),pipe=pipe.ToString(),space_ratio=(double)space.StepsPerTick.Value/belt.StepsPerTick.Value,mode.MinIslandLayer,mode.MaxIslandLayer,space_ids=mode.Islands.SpaceBelts.Select(x=>x.Id.ToString()).ToArray(),pipe_ids=mode.Islands.SpacePipes.Select(x=>x.Id.ToString()).ToArray()};
   case "capabilities":return new{game=Application.version,data=GameEnvironment.DataPath,simulator="Simulator",headless=true,operations=new[]{"capabilities","blueprint","catalog","capacities","recipe","inspect-save"}};
   case "blueprint":using(var world=new LabWorld())return world.Run(args,root);
   case "catalog":
    var definitions=new JObject();
    foreach(var obj in Resources.FindObjectsOfTypeAll<ScriptableObject>()){
     if(obj.GetType().Name.Contains("Metadata")||obj.GetType().Name.Contains("BuildingInternalVariant")){
      try{definitions[obj.name]=JObject.Parse(JsonUtility.ToJson(obj));}catch{}
     }
    }
    var output=Path.Combine(root,"definitions.json");File.WriteAllText(output,definitions.ToString());return new{path=output,count=definitions.Count};
   default:throw new ArgumentException("Unknown operation: "+operation);
  }
 }
}
