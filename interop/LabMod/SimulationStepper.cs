using System;
using System.Linq;
using Game.Core.Map.Simulation;
using Game.Core.Map.Simulation.Clustering;
using Game.Core.Simulation;

internal static class SimulationStepper
{
    internal static void Advance(MapModel model, int seconds, int ticksPerSecond,
        Action<Ticks, Ticks>? before = null, Action<Ticks, Ticks>? after = null, Func<bool>? stop = null)
    {
        if (seconds < 0 || ticksPerSecond <= 0)
            throw new ArgumentException("Invalid simulation duration or step");
        var count = checked(seconds * ticksPerSecond);
        var delta = Ticks.OneSecond / ticksPerSecond;
        if (delta <= Ticks.Zero)
            throw new ArgumentOutOfRangeException(nameof(ticksPerSecond));
        var lods = model.Islands.SelectMany(i => i.Definition.Layout.GetChunkPositions()
            .Select(c => SimulationLOD.Highest(c.ToGlobal(i.Transform)))).ToArray();
        var update = new SimulationUpdateConfiguration(lods, TimeSpan.Zero, 1);
        var strategy = new ProcessingUpdateStrategy(SimulationConstants.MaxSimulationUpdateDelta,
            SimulationConstants.MaxSimulatorIncrement, new[] { Ticks.Zero });
        var simulator = (Simulator)model.Simulator;
        for (var i = 0; i < count; i++)
        {
            var start = simulator.SimulationTime;
            before?.Invoke(start, delta);
            simulator.SynchronousUpdate(delta, update, strategy);
            after?.Invoke(simulator.SimulationTime, delta);
            if (stop?.Invoke() == true)
                break;
        }
    }
}
