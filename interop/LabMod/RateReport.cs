using System;
using System.Linq;
using Newtonsoft.Json;

internal sealed class ShapeLaneReport
{
    [JsonProperty("tile")]
    public (int, int, int) Tile
    {
        get;
    }
    [JsonProperty("rate")]
    public double Rate
    {
        get;
    }
    [JsonProperty("ratio")]
    public double Ratio
    {
        get;
    }
    internal ShapeLaneReport(TilePosition tile, double rate, double ratio)
    {
        Tile = tile.ToTuple();
        Rate = rate;
        Ratio = ratio;
    }
}

internal sealed class FluidLaneReport
{
    [JsonProperty("tile")]
    public (int, int, int) Tile
    {
        get;
    }
    [JsonProperty("liters_per_second")]
    public double LitersPerSecond
    {
        get;
    }
    [JsonProperty("ratio")]
    public double Ratio
    {
        get;
    }
    internal FluidLaneReport(TilePosition tile, double rate, double ratio)
    {
        Tile = tile.ToTuple();
        LitersPerSecond = rate;
        Ratio = ratio;
    }
}

internal sealed class RateReport
{
    [JsonProperty("window_seconds")]
    public int WindowSeconds
    {
        get;
    }
    [JsonProperty("expected_items_per_second")]
    public double ExpectedItemsPerSecond
    {
        get;
    }
    [JsonProperty("expected_liters_per_second")] public double ExpectedLitersPerSecond => ExpectedItemsPerSecond * 10;
    [JsonProperty("min_shape_ratio")]
    public double MinShapeRatio
    {
        get;
    }
    [JsonProperty("min_fluid_ratio")]
    public double MinFluidRatio
    {
        get;
    }
    [JsonProperty("shape_lanes")]
    public ShapeLaneReport[] ShapeLanes
    {
        get;
    }
    [JsonProperty("fluid_lanes")]
    public FluidLaneReport[] FluidLanes
    {
        get;
    }

    internal RateReport(int seconds, double expectedRate, ShapeLaneReport[] shapes, FluidLaneReport[] fluids)
    {
        WindowSeconds = seconds;
        ExpectedItemsPerSecond = expectedRate;
        ShapeLanes = shapes;
        FluidLanes = fluids;
        MinShapeRatio = shapes.Length == 0 ? 1 : shapes.Min(x => x.Ratio);
        MinFluidRatio = fluids.Length == 0 ? 1 : fluids.Min(x => x.Ratio);
    }
}
