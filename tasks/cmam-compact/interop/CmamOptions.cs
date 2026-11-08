using System;
using System.Linq;

internal sealed class CmamOptions
{
    internal bool Adapter
    {
        get;
    }
    internal int Seconds
    {
        get;
    }
    internal string Name
    {
        get;
    }
    internal long? MinimumProduced
    {
        get;
    }
    internal ProductionOptions? Production
    {
        get;
    }

    internal CmamOptions(LabArguments args)
    {
        Adapter = args.AdapterProbe ?? false;
        Seconds = ArgumentChecks.Nonnegative(args.Seconds ?? 1, "seconds");
        Name = ArgumentChecks.CaseName(args, "production");
        MinimumProduced = args.MinProduced;
        if (MinimumProduced < 0)
            throw new ArgumentOutOfRangeException("min_produced");
        var manifest = args.ProductionManifest;
        if (manifest != null)
        {
            if (Adapter)
                throw new ArgumentException("Production experiment cannot cut adapter buildings");
            Production = new ProductionOptions(manifest, ArgumentChecks.Required(args.Target, "target"));
        }
    }
}

internal sealed class ProductionOptions
{
    internal SupplyManifest Supply
    {
        get;
    }
    internal string Target
    {
        get;
    }
    internal int OutputX
    {
        get;
    }
    internal int OutputY
    {
        get;
    }
    internal int SignalReceivers
    {
        get;
    }

    internal ProductionOptions(ManifestInput manifest, string target)
    {
        Supply = new SupplyManifest(manifest);
        Target = target;
        var production = ArgumentChecks.Required(manifest.Production, "production");
        var chunk = production.OutputChunk;
        OutputX = checked(chunk.X * 20);
        OutputY = checked(chunk.Y * 20);
        SignalReceivers = ArgumentChecks.Nonnegative(production.SignalReceivers, "signal_receivers");
    }
}
