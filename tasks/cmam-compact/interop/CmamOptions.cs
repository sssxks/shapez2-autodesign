using System;
using Newtonsoft.Json.Linq;

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

    internal CmamOptions(JObject args)
    {
        Adapter = JsonInput.Boolean(args, "adapter_probe") ?? false;
        Seconds = JsonInput.Nonnegative(JsonInput.Integer(args, "seconds") ?? 1, "seconds");
        Name = JsonInput.CaseName(args, "production");
        MinimumProduced = JsonInput.Long(args, "min_produced");
        if (MinimumProduced < 0)
            throw new ArgumentOutOfRangeException("min_produced");
        var path = JsonInput.OptionalString(args, "production_manifest");
        if (path != null)
        {
            if (Adapter)
                throw new ArgumentException("Production experiment cannot cut adapter buildings");
            Production = new ProductionOptions(JsonInput.ReadFile(path), JsonInput.String(args, "target"));
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

    internal ProductionOptions(JObject manifest, string target)
    {
        Supply = new SupplyManifest(manifest);
        Target = target;
        var production = JsonInput.Object(JsonInput.Required(manifest, "production"), "production");
        var chunk = JsonInput.Array(production, "output_chunk");
        if (chunk.Count != 2 || chunk[0].Type != JTokenType.Integer || chunk[1].Type != JTokenType.Integer)
            throw new ArgumentException("output_chunk must contain two integer coordinates");
        OutputX = checked((int)chunk[0] * 20);
        OutputY = checked((int)chunk[1] * 20);
        SignalReceivers = JsonInput.Nonnegative(JsonInput.Integer(production, "signal_receivers")
            ?? throw new ArgumentException("signal_receivers is required"), "signal_receivers");
    }
}
