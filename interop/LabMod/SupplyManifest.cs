using System;
using System.Collections.Generic;
using Newtonsoft.Json.Linq;

public sealed class SupplyManifest
{
    public IReadOnlyDictionary<ChunkPosition, string> Sources
    {
        get;
    }

    public SupplyManifest(JObject input)
    {
        var sources = new Dictionary<ChunkPosition, string>();
        foreach (var value in JsonInput.Array(input, "sources"))
        {
            var source = JsonInput.Object(value, "source");
            var p = JsonInput.Coordinate(JsonInput.Required(source, "chunk"), "chunk");
            sources.Add(new ChunkPosition(p.X, p.Y, p.Z), JsonInput.String(source, "item"));
        }
        Sources = sources;
    }
}

internal sealed class RateManifest
{
    internal SupplyManifest Supply
    {
        get;
    }
    internal IReadOnlyDictionary<TilePosition, string> Targets
    {
        get;
    }
    internal HashSet<ChunkPosition> CutIslands
    {
        get;
    }

    internal RateManifest(JObject input)
    {
        Supply = new SupplyManifest(input);
        var targets = new Dictionary<TilePosition, string>();
        foreach (var value in JsonInput.Array(input, "targets"))
        {
            var target = JsonInput.Object(value, "target");
            var p = JsonInput.Coordinate(JsonInput.Required(target, "tile"), "tile");
            targets.Add(new TilePosition(p.X, p.Y, p.Z), JsonInput.String(target, "item"));
        }
        if (targets.Count == 0)
            throw new ArgumentException("Rate measurement requires targets");
        Targets = targets;
        CutIslands = new HashSet<ChunkPosition>();
        foreach (var value in JsonInput.Array(input, "cut_islands"))
        {
            var p = JsonInput.Coordinate(value, "cut_islands");
            CutIslands.Add(new ChunkPosition(p.X, p.Y, p.Z));
        }
    }
}
