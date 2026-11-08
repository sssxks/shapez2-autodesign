using System;

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

    internal BlueprintRunOptions(LabArguments input)
    {
        Path = ArgumentChecks.Required(input.Blueprint, "blueprint");
        Name = ArgumentChecks.CaseName(input, "blueprint");
        Seconds = ArgumentChecks.Nonnegative(input.Seconds ?? 1, "seconds");
        ExpectedBuildings = input.ExpectedBuildings;
        if (ExpectedBuildings is int count)
            ArgumentChecks.Nonnegative(count, "expected_buildings");
        ExportGraph = input.ExportGraph ?? true;
        var rate = input.RateManifest;
        Rate = rate == null ? null : new RateOptions(input, rate, Seconds);
        Experiment = input.Experiment;
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

    internal RateOptions(LabArguments input, ManifestInput manifest, int seconds)
    {
        if (seconds <= 0)
            throw new ArgumentOutOfRangeException("seconds", "Rate window must be positive");
        Seconds = seconds;
        WarmupSeconds = ArgumentChecks.Nonnegative(input.WarmupSeconds ?? 300, "warmup_seconds");
        MinimumRatio = input.MinimumRateRatio ?? 0.99;
        if (MinimumRatio < 0 || MinimumRatio > 1)
            throw new ArgumentOutOfRangeException("minimum_rate_ratio", "Must be between zero and one");
        Manifest = new RateManifest(manifest);
    }
}

