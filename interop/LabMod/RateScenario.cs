using System;
using System.IO;
using Newtonsoft.Json.Linq;

internal static class RateScenario
{
    internal static object Run(LabWorld world, RateProbe probe, JObject manifest, JObject args, string root)
    {
        var seconds = (int?)args["seconds"] ?? 1;
        var warmup = (int?)args["warmup_seconds"] ?? 300;
        if (seconds <= 0 || warmup < 0)
            throw new ArgumentException("Rate measurement needs a positive window and nonnegative warmup");
        probe.Attach(world.Simulator, manifest);
        world.Advance(warmup, 10, probe.BeforeTick, probe.AfterTick);
        probe.ResetMeters();
        world.Advance(seconds, 10, probe.BeforeTick, probe.AfterTick);
        var report = probe.Report(seconds);
        File.WriteAllText(Path.Combine(root, "rate-report.json"), report.ToString());
        var minimum = (double?)args["minimum_rate_ratio"] ?? 0.99;
        if ((double)report["min_shape_ratio"] < minimum || (double)report["min_fluid_ratio"] < minimum)
            throw new Exception("Full-rate test failed: " + report);
        return report;
    }
}
