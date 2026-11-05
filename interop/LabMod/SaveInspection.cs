using System;
using System.Collections.Generic;
using System.Linq;
using Game.Core.Map.Simulation;
using Game.Core.Trains;
using Game.Content.Features.Fluids;
using Game.Core.Simulation;
using Game.Content.AtomicBuildings.WireGlobalTransmitters.Receiver;
using Game.Content.Features.Signals.Channels;
using Game.Content.Features.Signals.Tick;
using Newtonsoft.Json.Linq;

internal static class SaveInspection
{
    internal static object Read(JObject args)
    {
        if (string.IsNullOrEmpty(Environment.GetEnvironmentVariable("SHAPEZ_LAB_SAVE")))
            throw new InvalidOperationException("Set SHAPEZ_LAB_SAVE to inspect a save copy");
        var session = GameBindings.Session;
        var map = GameBindings.Field<MapModel>(session, "MapModel");
        var simulator = map.Simulator;
        var seconds = (int?)args["seconds"] ?? 0;
        if (seconds > 0)
        {
            var lods = map.Islands.SelectMany(i => i.Definition.Layout.GetChunkPositions().Select(c => SimulationLOD.Highest(c.ToGlobal(i.Transform)))).ToArray();
            var update = new SimulationUpdateConfiguration(lods, TimeSpan.Zero, 1);
            var strategy = new ProcessingUpdateStrategy(SimulationConstants.MaxSimulationUpdateDelta, SimulationConstants.MaxSimulatorIncrement, new[] { Ticks.Zero });
            for (int i = 0; i < seconds * 10; i++)
                ((Simulator)simulator).SynchronousUpdate(Ticks.OneSecond / 10, update, strategy);
        }
        var channels = GameBindings.Field<SignalChannelRegistry>(session, "SignalChannelRegistry");
        var shapesRegistry = GameBindings.Shapes;
        var receivers = new List<object>();
        var unloaders = new List<object>();
        foreach (var localized in simulator.Simulations)
        {
            var s = localized.Simulation;
            var p = localized.GetOccupiedChunk(0);
            if (s is ControlledSignalReceiverSimulation controlled)
                receivers.Add(new
                {
                    chunk = new[] { p.x, p.y, (int)p.z },
                    channel = controlled.LastControlSignal.ToString(),
                    signal = controlled.LastOutputSignal.ToString()
                });
            if (s is ConstantSignalSimulation constant)
            {
                var config = GameBindings.Field<ConstantSignalConfiguration>(constant, "Config");
                receivers.Add(new
                {
                    chunk = new[] { p.x, p.y, (int)p.z },
                    channel = "constant",
                    signal = config.Value.ToString()
                });
            }
            if (s is GlobalSignalReceiverSimulation receiver)
            {
                var config = GameBindings.Field<IGlobalSignalReceiverConfiguration>(receiver, "Config");
                var signal = channels.TryGetReadOnlyChannel(config.ChannelId, out var channel) ? channel.ReadSignal(SignalTicks.FromTicks(simulator.SimulationTime)).ToString() : null;
                receivers.Add(new
                {
                    chunk = new[] { p.x, p.y, (int)p.z },
                    channel = config.ChannelId.ToString(),
                    signal
                });
            }
            if (s is TrainCargoUnloaderSimulation<ShapeId> shapes)
                unloaders.Add(new
                {
                    chunk = new[] { p.x, p.y, (int)p.z },
                    type = "shape",
                    packages = GameBindings.ShapeCargo(shapes).Select(f => new { f.Package.Amount, item = f.Package.Item.IsValid ? shapesRegistry.GetItem(f.Package.Item).ToString() : null }).ToArray()
                });
            if (s is TrainCargoUnloaderSimulation<FluidId> fluids)
                unloaders.Add(new
                {
                    chunk = new[] { p.x, p.y, (int)p.z },
                    type = "fluid",
                    packages = GameBindings.FluidCargo(fluids).Select(f => JToken.FromObject(f.Package)).ToArray()
                });
        }
        return new
        {
            map.BuildingCount,
            map.IslandCount,
            receivers,
            unloaders,
            simulation_types = simulator.Simulations.GroupBy(s => s.Simulation.GetType().FullName).ToDictionary(g => g.Key, g => g.Count())
        };
    }
}
