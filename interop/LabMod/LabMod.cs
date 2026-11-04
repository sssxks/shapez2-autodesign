using System;
using System.IO;
using System.Diagnostics;
using Newtonsoft.Json;
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
 private string root;private bool ready;
 void Awake(){root=Environment.GetEnvironmentVariable("SHAPEZ_LAB_RUN");Application.runInBackground=true;}
 void Update(){try{
  if(File.Exists(Path.Combine(root,"shutdown"))){Application.Quit();return;}
  if(!ready){
   if(!StaticGameCoreAccessor.HasInstance())return;
   if(StaticGameCoreAccessor.G.LocalPlayer.CurrentMap==null)return;
   Write("ready.json",new{pid=Process.GetCurrentProcess().Id,data_path=GameEnvironment.DataPath,version=Application.version});ready=true;
  }
  var path=Path.Combine(root,"request.json");if(!File.Exists(path))return;
  var request=JObject.Parse(File.ReadAllText(path));File.Delete(path);var id=(int)request["id"];
  try{var result=Execute((string)request["operation"],(JObject)request["args"]);Write("result-"+id+".json",new{id,ok=true,result});}
  catch(Exception e){Write("result-"+id+".json",new{id,ok=false,error=e.ToString()});}
 }catch(Exception e){Write("fatal.json",new{error=e.ToString()});enabled=false;}}
 private object Execute(string operation,JObject args){
  switch(operation){
   case "capabilities":return new{game=Application.version,data=GameEnvironment.DataPath,simulator=StaticGameCoreAccessor.G.LocalPlayer.CurrentMap.Simulator.GetType().FullName,operations=new[]{"capabilities","blueprint","catalog"}};
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
 private void Write(string name,object value){var dest=Path.Combine(root,name);var temp=dest+".tmp";File.WriteAllText(temp,JsonConvert.SerializeObject(value,Formatting.Indented));File.Move(temp,dest);}
}
