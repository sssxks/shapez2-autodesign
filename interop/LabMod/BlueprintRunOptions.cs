using System;
using Newtonsoft.Json.Linq;

internal sealed class BlueprintRunOptions
{
    internal string Path
    {
        get;
    }
    internal string Name
    {
        get;
    }
    internal int Seconds
    {
        get;
    }
    internal int? ExpectedBuildings
    {
        get;
    }
    internal bool ExportGraph
    {
        get;
    }
    internal RateOptions? Rate
    {
        get;
    }
    internal ExperimentDefinition? Experiment
    {
        get;
    }

    internal BlueprintRunOptions(JObject input)
    {
        Path = JsonInput.String(input, "blueprint");
        Name = JsonInput.CaseName(input, "blueprint");
        Seconds = JsonInput.Nonnegative(JsonInput.Integer(input, "seconds") ?? 1, "seconds");
        ExpectedBuildings = JsonInput.Integer(input, "expected_buildings");
        if (ExpectedBuildings is int count)
            JsonInput.Nonnegative(count, "expected_buildings");
        ExportGraph = JsonInput.Boolean(input, "export_graph") ?? true;
        var ratePath = JsonInput.OptionalString(input, "rate_manifest");
        Rate = ratePath == null ? null : new RateOptions(input, ratePath, Seconds);
        Experiment = input["experiment"] is JToken definition && definition.Type != JTokenType.Null
            ? new ExperimentDefinition(JsonInput.Object(definition, "experiment")) : null;
        if (Rate != null && Experiment != null)
            throw new ArgumentException("Choose either rate_manifest or experiment");
    }
}

internal sealed class RateOptions
{
    internal RateManifest Manifest
    {
        get;
    }
    internal int Seconds
    {
        get;
    }
    internal int WarmupSeconds
    {
        get;
    }
    internal double MinimumRatio
    {
        get;
    }

    internal RateOptions(JObject input, string path, int seconds)
    {
        if (seconds <= 0)
            throw new ArgumentOutOfRangeException("seconds", "Rate window must be positive");
        Seconds = seconds;
        WarmupSeconds = JsonInput.Nonnegative(JsonInput.Integer(input, "warmup_seconds") ?? 300, "warmup_seconds");
        MinimumRatio = JsonInput.Number(input, "minimum_rate_ratio") ?? 0.99;
        if (MinimumRatio < 0 || MinimumRatio > 1)
            throw new ArgumentOutOfRangeException("minimum_rate_ratio", "Must be between zero and one");
        Manifest = new RateManifest(JsonInput.ReadFile(path));
    }
}

internal sealed class ExperimentDefinition
{
    internal string Assembly
    {
        get;
    }
    internal string Type
    {
        get;
    }
    internal ExperimentDefinition(JObject input)
    {
        Assembly = JsonInput.String(input, "assembly");
        Type = JsonInput.String(input, "type");
    }
}
