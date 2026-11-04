using System;
using System.Reflection;
using Game.Orchestration;

public static class GameBindings {
 public static object Session {
  get{if(LabBootstrap.Session!=null)return LabBootstrap.Session;var orchestrator=typeof(GameBootstrapper).GetField("GameOrchestrator",BindingFlags.Static|BindingFlags.NonPublic).GetValue(null);
   var session=Field<object>(orchestrator,"CurrentSubOrchestrator");if(!(session is GameSessionOrchestrator))throw new InvalidOperationException("Game session is not ready");return session;}
 }
 public static T Field<T>(object obj,string name)=>(T)obj.GetType().GetField(name,BindingFlags.NonPublic|BindingFlags.Public|BindingFlags.Instance).GetValue(obj);
}
