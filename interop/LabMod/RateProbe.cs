using System;
using System.Collections.Generic;
using System.Linq;
using Game.Content.BuildingPath.Simulation;
using Game.Content.Features.Fluids;
using Game.Core.Coordinates;
using Game.Core.Map.Simulation;
using Game.Core.Simulation;
using Game.Core.Trains;

internal sealed class RateProbe
{
    private readonly UnloaderSupply supply;
    private readonly Dictionary<TilePosition, Meter> shapes = new Dictionary<TilePosition, Meter>();
    private readonly Dictionary<TilePosition, (ProvidingFluidContainer container, string expected)> fluids = new Dictionary<TilePosition, (ProvidingFluidContainer, string)>();
    private readonly Dictionary<TilePosition, long> volumes = new Dictionary<TilePosition, long>();
    private readonly double expectedBeltRate;
    sealed class Meter : IItemReceiver
    {
        internal long Count;
        private readonly string expected;
        private readonly Ticks interval;
        private Ticks credit;
        internal Meter(string expected, double rate)
        {
            this.expected = expected;
            interval = Ticks.FromSeconds((float)(1 / rate));
        }
        internal void Supply(Ticks delta)
        {
            credit += delta;
            if (credit > interval)
                credit = interval;
        }
        public Steps MaxStep_S => LaneConstants.ItemSpacing;
        public bool CanAcceptItem(IBeltItem item) => credit >= interval;
        public void HandOverItem(IBeltItem item, Ticks remaining)
        {
            if (item.ToString() != expected)
                throw new Exception("Wrong shape: " + item + ", expected " + expected);
            credit -= interval;
            Count++;
        }
    }
    internal RateProbe(ISimulator simulator, RateManifest manifest)
    {
        supply = new UnloaderSupply(simulator, manifest.Supply);
        var mode = GameBindings.Mode;
        BeltSpeed speed = mode.Buildings.ForwardBelt.ConfigAs<IConveyorConfiguration>().ConveyorSpeed;
        expectedBeltRate = (speed.StepsPerTick * Ticks.OneSecond).Value / (double)LaneConstants.ItemSpacing.Value;
        foreach (var localized in simulator.Simulations)
        {
            var sim = localized.Simulation;
            if (!(localized is ILocalizedTileSimulation tile))
                continue;
            for (int i = 0; i < tile.NumOccupiedTiles; i++)
            {
                var p = tile.GetOccupiedTile(i);
                var key = new TilePosition(p.x, p.y, p.z);
                if (!manifest.Targets.TryGetValue(key, out var expected))
                    continue;
                if (expected.Length > 1 && sim is IItemSimulation belt && belt.NumItemProviders == 1)
                {
                    var meter = new Meter(expected, expectedBeltRate);
                    belt.GetItemProvider(0).NextLane = meter;
                    shapes.Add(key, meter);
                }
                if (expected.Length == 1 && sim is ISimulationComposedWithFluidPortReceiver fluid)
                {
                    fluids.Add(key, (fluid.FluidPortReceiverContainer, expected));
                    volumes.Add(key, 0);
                }
            }
        }
        var expectedShapes = manifest.Targets.Count(p => p.Value.Length > 1);
        var expectedFluids = manifest.Targets.Count - expectedShapes;
        if (shapes.Count != expectedShapes || fluids.Count != expectedFluids)
            throw new Exception($"Endpoint mismatch: shape lanes {shapes.Count}/{expectedShapes}, fluid lanes {fluids.Count}/{expectedFluids}");
    }
    internal void BeforeTick(Ticks start, Ticks delta)
    {
        supply.BeforeTick();
        foreach (var meter in shapes.Values)
            meter.Supply(delta);
    }
    internal void AfterTick(Ticks end, Ticks delta)
    {
        foreach (var pair in fluids)
        {
            var c = pair.Value.container;
            if (c.IsEmpty)
                continue;
            if ((c.Fluid as ColorFluid)?.Color.Code.ToString() != pair.Value.expected)
                throw new Exception("Wrong fluid at " + pair.Key);
            volumes[pair.Key] += c.Value.Units;
            c.Take(c.Value);
        }
    }
    internal void ResetMeters()
    {
        foreach (var m in shapes.Values)
            m.Count = 0;
        foreach (var key in volumes.Keys.ToArray())
            volumes[key] = 0;
    }
    internal RateReport Report(int seconds)
    {
        var s = shapes.Select(p => new ShapeLaneReport(p.Key, p.Value.Count / (double)seconds,
            p.Value.Count / (seconds * expectedBeltRate))).ToArray();
        var f = volumes.Select(p => new FluidLaneReport(p.Key,
            p.Value / (double)FluidUnit.UnitsPerLiter / seconds,
            p.Value / (double)FluidUnit.UnitsPerLiter / seconds / (expectedBeltRate * 10))).ToArray();
        return new RateReport(seconds, expectedBeltRate, s, f);
    }
}
