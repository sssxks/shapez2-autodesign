using System;
using System.Reflection;
using Game.Core.Coordinates;
using Newtonsoft.Json.Linq;

/// Task assemblies own experimental policy; the host owns the native world.
public interface IBlueprintExperiment {
 void Configure(JObject args);
 bool KeepBuilding(GlobalTileCoordinate position);
 object Run(LabWorld world,JObject args,string root);
}

internal static class BlueprintExperiment {
 internal static IBlueprintExperiment Load(JObject definition){
  if(definition==null)return null;
  var assembly=Assembly.LoadFrom((string)definition["assembly"]);
  var type=assembly.GetType((string)definition["type"],true);
  return (IBlueprintExperiment)Activator.CreateInstance(type);
 }
}
