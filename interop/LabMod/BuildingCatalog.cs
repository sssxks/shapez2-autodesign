using System;
using System.Linq;
using Game.Core.Coordinates;

public sealed class BuildingCapability
{
    public string Id
    {
        get;
    }
    public string[] Tiles
    {
        get;
    }
    public ConnectorCapability[] Connectors
    {
        get;
    }
    public string? CutterSpeed
    {
        get;
    }
    public string? CutterDelay
    {
        get;
    }
    internal BuildingCapability(string id, string[] tiles, ConnectorCapability[] connectors, string? speed, string? delay)
    {
        Id = id;
        Tiles = tiles;
        Connectors = connectors;
        CutterSpeed = speed;
        CutterDelay = delay;
    }
}
public sealed class ConnectorCapability
{
    public string Kind
    {
        get;
    }
    public string Pivot
    {
        get;
    }
    internal ConnectorCapability(string kind, string pivot)
    {
        Kind = kind;
        Pivot = pivot;
    }
}
public static class BuildingCatalog
{
    public static BuildingCapability[] Query(Func<string, bool>? include = null) =>
        GameBindings.Mode.Buildings.All.SelectMany(group => group.Definitions)
            .Where(definition => include == null || include(definition.Id.ToString()))
            .Select(definition =>
            {
                var id = definition.Id.ToString();
                var connectors = definition.CustomData.Get<IBuildingConnectorData>();
                var cutter = id.Contains("Cutter") && !id.StartsWith("Virtual");
                return new BuildingCapability(id,
                    connectors.Tiles.Select(tile => tile.ToString()).ToArray(),
                    connectors.AllBuildingConnectors.Select(io => new ConnectorCapability(io.GetType().Name, io.Pivot().ToString())).ToArray(),
                    cutter ? definition.ConfigAs<ICutterConfiguration>().BeltSpeed.ToString() : null,
                    cutter ? definition.ConfigAs<ICutterConfiguration>().ProcessingDelay.ToString() : null);
            }).OrderBy(definition => definition.Id, StringComparer.Ordinal).ToArray();
}
