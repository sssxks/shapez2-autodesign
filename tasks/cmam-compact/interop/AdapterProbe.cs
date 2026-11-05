using System;
using System.Collections.Generic;
using System.Linq;
using Game.Core.Coordinates;
using Game.Core.Map.Simulation;
using Game.Core.Simulation;
using Game.Content.Features.Fluids;
using Game.Content.Features.SpacePaths;

// Terminate the DUT at CMAM's physical input receivers. Production circuitry is
// intentionally outside this test: the assertion is transport identity at every lane.
internal sealed class AdapterProbe
{
    internal static readonly Dictionary<(int, int, int), string> Targets = CreateTargets();
    static Dictionary<(int, int, int), string> CreateTargets()
    {
        var t = new Dictionary<(int, int, int), string>();
        for (int j = 0; j < 5; j++)
            for (int z = 0; z < 3; z++)
                for (int k = 0; k < 4; k++)
                {
                    foreach (var p in new[] { (-19, 122, "SuSuSuSu"), (-18, 122, "P-P-P-P-"), (-17, 122, "WuWuWuWu"), (-19, 177, "CuCuCuCu"), (-17, 177, "RuRuRuRu") })
                        t.Add(((p.Item1 + 5 * j) * 20 + 8 + k, p.Item2, z), p.Item3);
                    foreach (var p in new[] { (-16, 122), (-15, 122), (-18, 177) })
                        t.Add(((p.Item1 + 5 * j) * 20 + 8 + k, p.Item2, z), "bgr"[z].ToString());
                }
        return t;
    }
    internal static bool KeepBuilding(GlobalTileCoordinate p) => p.x < -380 || p.x >= 200 || p.y < 120 || p.y >= 180 || Targets.ContainsKey((p.x, p.y, p.z));
    sealed class Sink : IItemReceiver
    {
        internal int Count; readonly string expected; readonly IItemReceiver lane;
        internal Sink(string expected, IItemReceiver lane)
        {
            this.expected = expected;
            this.lane = lane;
        }
        public Steps MaxStep_S => lane.MaxStep_S;
        public bool CanAcceptItem(IBeltItem item) => true;
        public void HandOverItem(IBeltItem item, Ticks remainingTicks)
        {
            if (item.ToString() != expected)
                throw new Exception("Wrong shape: expected " + expected + ", got " + item);
            Count++;
        }
    }
    readonly List<(IItemReceiver lane, IBeltItem item)> sources = new List<(IItemReceiver, IBeltItem)>();
    readonly Dictionary<(int, int, int), Sink> shapeSinks = new Dictionary<(int, int, int), Sink>();
    readonly Dictionary<(int, int, int), (ProvidingFluidContainer container, string expected)> fluidSinks = new Dictionary<(int, int, int), (ProvidingFluidContainer, string)>();
    readonly HashSet<(int, int, int)> fluidsSeen = new HashSet<(int, int, int)>();
    int injected;
    internal AdapterProbe(ISimulator simulator)
    {
        var session = GameBindings.Session;
        var registry = GameBindings.Shapes;
        var ids = GameBindings.ShapeIds;
        var fluids = GameBindings.Colors;
        foreach (var localized in simulator.Simulations)
        {
            var sim = localized.Simulation;
            if (sim.GetType().Name.StartsWith("TrainCargoUnloaderSimulation"))
            {
                var x = localized.GetOccupiedChunk(0).x;
                IBeltItem item;
                if (x >= 14 && x <= 18)
                {
                    var hashes = new[] { "P-P-P-P-", "CuCuCuCu", "RuRuRuRu", "SuSuSuSu", "WuWuWuWu" };
                    item = registry.GetItem(ids.Resolve(hashes[x - 14]));
                }
                else if (x >= 19 && x <= 21)
                {
                    var color = "rgb"[x - 19];
                    item = new FluidPackageItem(fluids.Single(f => f.Color.Code == color), FluidUnit.FromLiters(10));
                }
                else
                    continue;
                var bundle = ((IItemBundleSimulation)sim).GetItemProviderBundle(0);
                for (short z = 0; z < 3; z++)
                    for (short k = 0; k < 4; k++)
                    {
                        var lane = bundle.GetSender(k, z).NextLane;
                        if (lane == null)
                            throw new Exception("Disconnected train outlet " + x);
                        sources.Add((lane, item));
                    }
            }
            if (!(localized is ILocalizedTileSimulation tile))
                continue;
            for (int i = 0; i < tile.NumOccupiedTiles; i++)
            {
                var p = tile.GetOccupiedTile(i);
                var key = (p.x, p.y, (int)p.z);
                if (!Targets.TryGetValue(key, out var expected))
                    continue;
                if (sim is BeltPortTransferSimulation belt)
                {
                    var sink = new Sink(expected, belt.GetItemReceiver(0));
                    belt.GetItemProvider(0).NextLane = sink;
                    shapeSinks.Add(key, sink);
                }
                if (sim is FluidPortTransferSimulation fluid)
                    fluidSinks.Add(key, (fluid.FluidPortReceiverContainer, expected));
            }
        }
        if (sources.Count != 96 || shapeSinks.Count != 300 || fluidSinks.Count != 180)
            throw new Exception($"Adapter endpoints missing: sources={sources.Count}/96 shapes={shapeSinks.Count}/300 fluids={fluidSinks.Count}/180");
    }
    internal void BeforeTick()
    {
        foreach (var source in sources)
            if (source.lane.CanAcceptItem(source.item))
            {
                source.lane.HandOverItem(source.item, Ticks.Zero);
                injected++;
            }
    }
    internal bool AfterTick()
    {
        foreach (var p in fluidSinks)
        {
            var c = p.Value.container;
            if (c.IsEmpty)
                continue;
            var color = (c.Fluid as ColorFluid)?.Color.Code.ToString();
            if (color != p.Value.expected)
                throw new Exception($"Wrong fluid at {p.Key}: expected {p.Value.expected}, got {color}");
            fluidsSeen.Add(p.Key);
            c.Take(c.Value);
        }
        return shapeSinks.Values.All(s => s.Count > 0) && fluidsSeen.Count == 180;
    }
    internal object Report() => new { injected, shape_lanes = shapeSinks.Count(p => p.Value.Count > 0), fluid_lanes = fluidsSeen.Count, expected_shape_lanes = 300, expected_fluid_lanes = 180, missing_shapes = shapeSinks.Where(p => p.Value.Count == 0).Select(p => p.Key.ToString()).ToArray(), missing_fluids = fluidSinks.Keys.Where(p => !fluidsSeen.Contains(p)).Select(p => p.ToString()).ToArray() };
}
