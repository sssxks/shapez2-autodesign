using System;
using Game.Blueprints;
using Game.Orchestration;
using System.Collections.Generic;
using System.Reflection;
using Core.Logging;
using Cysharp.Threading.Tasks;
using Game.Core.Blueprint.Importer;
using Game.Core.Blueprint.Importer.Migration;
using Game.Core.Blueprint.Exporter;
using Game.Core.Blueprints;
using Game.Core.Blueprint;

// Called only by the patched temporary game clone. Retain native mode/content
// initialization; omit player interaction, audio, renderers, HUD and menu ticking.
public static class LabBootstrap {
 internal static object Session;
 internal static IBlueprintImporter Importer;
 public static UniTask Prepare(object session,object options,object globals,object gameData,object platform,object content,object metadata){
  Invoke(session,"Init_1_GameOptions",globals,options,gameData,platform);
  Invoke(session,"Init_2_Player");
  Invoke(session,"Init_3_SavegameAndMode",content,gameData,options);
  var logger=LabWorld.Field<ILogger>(session,"Logger");var mode=LabWorld.Field<GameMode>(session,"Mode");
  var migrators=new BlueprintMigrators(logger);
  var factory=new BlueprintImporterFactory(new CustomDataConverter(new List<Game.Blueprints.IBlueprintAdditionalDataConverter>{new BlueprintIconConverter()}));
  Importer=factory.CreateImporter(mode.Buildings,mode.Islands,migrators.FormatMigrators,migrators.CandidateMigrators);
  foreach(var camera in UnityEngine.Resources.FindObjectsOfTypeAll<UnityEngine.Camera>())camera.enabled=false;
  UnityEngine.AudioListener.pause=true;UnityEngine.QualitySettings.vSyncCount=0;
  Session=session;return UniTask.CompletedTask;
 }
 static void Invoke(object o,string name,params object[] args){try{o.GetType().GetMethod(name,BindingFlags.Instance|BindingFlags.NonPublic).Invoke(o,args);}catch(TargetInvocationException e){throw e.InnerException??e;}}
}
