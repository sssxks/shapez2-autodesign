using System;
using System.Reflection;
using Game.Core.Coordinates;
using Newtonsoft.Json.Linq;

/// Task assemblies own experimental policy; the host owns the native world.
public interface IBlueprintExperiment
{
    void Configure(JObject args);
    bool KeepBuilding(GlobalTileCoordinate position);
    object Run(LabWorld world, JObject args, string root);
}

internal static class BlueprintExperiment
{
    internal static IBlueprintExperiment? Load(ExperimentDefinition? definition)
    {
        if (definition == null)
            return null;
        var assembly = Assembly.LoadFrom(definition.Assembly);
        var type = assembly.GetType(definition.Type, true);
        if (!typeof(IBlueprintExperiment).IsAssignableFrom(type) || type.IsAbstract)
            throw new ArgumentException($"{definition.Type} must implement IBlueprintExperiment");
        return Activator.CreateInstance(type) as IBlueprintExperiment
            ?? throw new InvalidOperationException($"Cannot create experiment {definition.Type}");
    }
}
