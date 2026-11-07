using System.IO;
using System.Linq;
using Game.Core.Coordinates;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

/// Export the active session's resolved definitions, rather than island metadata.
public sealed class QuarterCutDefinitions : IBlueprintExperiment
{
    public void Configure(JObject args) { }
    public bool KeepBuilding(GlobalTileCoordinate position) => true;
    public object Run(LabWorld world, JObject args, string root)
    {
        var definitions = GameBindings.Mode.Buildings.All
            .SelectMany(group => group.Definitions)
            .Where(definition => !definition.Id.ToString().StartsWith("Virtual"))
            .Where(definition => definition.Id.ToString().Contains("Cutter") || definition.Id.ToString().Contains("Rotator"))
            .Select(definition => new {
                id = definition.Id.ToString(),
                connectors = definition.CustomData.Get<IBuildingConnectorData>().Tiles.Select(tile => tile.ToString()).ToArray(),
                configuration = definition.Id.ToString().Contains("Cutter") ? (object)new { speed = definition.ConfigAs<ICutterConfiguration>().BeltSpeed.ToString(), delay = definition.ConfigAs<ICutterConfiguration>().ProcessingDelay.ToString() } : null
            }).ToArray();
        var path = Path.Combine(root, "quarter-cut-definitions.json");
        File.WriteAllText(path, JsonConvert.SerializeObject(definitions, Formatting.Indented,
            new JsonSerializerSettings { ReferenceLoopHandling = ReferenceLoopHandling.Ignore }));
        return new { path, count = definitions.Length };
    }
}


