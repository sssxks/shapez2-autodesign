using System;
using System.Collections.Generic;
using Game.Core.Map.Simulation;
using Game.Core.Simulation;

/// Rate-limited supply with at most one retained item of credit under backpressure.
public sealed class BeltSource
{
    private readonly IItemReceiver lane;
    private double credit;
    public IBeltItem Item
    {
        get; set;
    }
    public BeltSource(IItemReceiver lane, string shape)
    {
        this.lane = lane;
        Item = GameBindings.Shapes.GetItem(GameBindings.ShapeIds.Resolve(shape));
    }
    public void Feed(double delta, double rate)
    {
        credit = Math.Min(1, credit + delta * rate);
        if (rate > 0 && credit >= 1 && lane.CanAcceptItem(Item))
        {
            lane.HandOverItem(Item, Ticks.Zero);
            credit -= 1;
        }
    }
}

/// Shape-resolved output counter with optional lane identity and backpressure.
public sealed class BeltMeter : IItemReceiver
{
    private readonly HashSet<string> accepted;
    public Dictionary<string, long> Counts { get; } = new Dictionary<string, long>();
    public bool Blocked
    {
        get; set;
    }
    public BeltMeter(params string[] acceptedShapes) => accepted = new HashSet<string>(acceptedShapes);
    public Steps MaxStep_S => LaneConstants.ItemSpacing;
    public bool CanAcceptItem(IBeltItem item) => !Blocked;
    public void HandOverItem(IBeltItem item, Ticks remainingTicks)
    {
        var key = item.ToString();
        if (accepted.Count != 0 && !accepted.Contains(key))
            throw new InvalidOperationException("Unexpected output shape or crossed lane: " + key);
        Counts.TryGetValue(key, out var count);
        Counts[key] = count + 1;
    }
}

/// Shared timing for task-defined belt acceptance phases.
public static class BeltMeasurement
{
    public static void Run(LabWorld world, int warmup, int seconds, IEnumerable<BeltMeter> meters, Action<Ticks, Ticks> feed)
    {
        world.Advance(warmup, 120, feed);
        foreach (var meter in meters)
            meter.Counts.Clear();
        world.Advance(seconds, 120, feed);
    }
}

