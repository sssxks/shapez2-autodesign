using System;
using System.Collections.Generic;

public sealed class SupplyManifest
{
    public IReadOnlyDictionary<ChunkPosition, string> Sources
    {
        get;
    }

    public SupplyManifest(ManifestInput input)
    {
        var sources = new Dictionary<ChunkPosition, string>();
        foreach (var source in ArgumentChecks.Required(input.Sources, "sources"))
        {
            var p = source.Position;
            sources.Add(new ChunkPosition(p.X, p.Y, p.Z), source.Item);
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

    internal RateManifest(ManifestInput input)
    {
        Supply = new SupplyManifest(input);
        var targets = new Dictionary<TilePosition, string>();
        foreach (var target in ArgumentChecks.Required(input.Targets, "targets"))
        {
            var p = target.Position;
            targets.Add(new TilePosition(p.X, p.Y, p.Z), target.Item);
        }
        if (targets.Count == 0)
            throw new ArgumentException("Rate measurement requires targets");
        Targets = targets;
        CutIslands = new HashSet<ChunkPosition>();
        foreach (var p in ArgumentChecks.Required(input.CutIslands, "cut_islands"))
        {
            CutIslands.Add(new ChunkPosition(p.X, p.Y, p.Z));
        }
    }
}
