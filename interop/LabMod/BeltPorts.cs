using System;
using System.Collections.Generic;
using Game.Content.BuildingPath.Simulation;
using Game.Core.Map.Simulation;
using Game.Core.Simulation;

/// Task-declared physical ports, parsed once before native execution.
public sealed class BeltPortManifest
{
    private readonly (string Role, int Lane, TilePosition Position, bool Output)[] ports;
    private BeltPortManifest((string, int, TilePosition, bool)[] ports) => this.ports = ports;
    public static BeltPortManifest Parse(LabArguments args)
    {
        var result = new List<(string, int, TilePosition, bool)>();
        var positions = new HashSet<TilePosition>();
        var identities = new HashSet<(string, int)>();
        foreach (var port in ArgumentChecks.Required(args.Ports, "ports"))
        {
            var role = port.Role;
            int lane = ArgumentChecks.Nonnegative(port.Lane, "lane");
            var p = port.Position;
            var position = new TilePosition(p.X, p.Y, p.Z);
            bool output = port.Output;
            if (!positions.Add(position) || !identities.Add((role, lane)))
                throw new ArgumentException("Duplicate physical port or lane identity");
            result.Add((role, lane, position, output));
        }
        return new BeltPortManifest(result.ToArray());
    }
    public BoundBeltPorts Bind(LabWorld world)
    {
        var receivers = new Dictionary<TilePosition, IItemReceiver>();
        var senders = new Dictionary<TilePosition, SpaceBeltPortSenderSimulation>();
        foreach (var simulation in ((ISimulator)world.Simulator).Simulations)
        {
            if (!(simulation is ILocalizedTileSimulation tile))
                continue;
            var p = tile.GetOccupiedTile(0);
            var position = new TilePosition(p.x, p.y, p.z);
            if (simulation.Simulation is SpaceBeltPortReceiverSimulation receiver)
                receivers.Add(position, receiver.InputLane);
            if (simulation.Simulation is SpaceBeltPortSenderSimulation sender)
                senders.Add(position, sender);
        }
        var inputs = new List<(string, int, TilePosition, IItemReceiver)>();
        var outputs = new List<(string, int, TilePosition, SpaceBeltPortSenderSimulation)>();
        foreach (var port in ports)
        {
            if (port.Output)
                outputs.Add((port.Role, port.Lane, port.Position, senders.TryGetValue(port.Position, out var sender) ? sender : throw new InvalidOperationException("Missing output at " + port.Position)));
            else
                inputs.Add((port.Role, port.Lane, port.Position, receivers.TryGetValue(port.Position, out var receiver) ? receiver : throw new InvalidOperationException("Missing input at " + port.Position)));
        }
        return new BoundBeltPorts(inputs, outputs);
    }
}

/// Native connections indexed by the design's stable role and lane identities.
public sealed class BoundBeltPorts
{
    public IReadOnlyList<(string Role, int Lane, TilePosition Position, IItemReceiver Receiver)> Inputs
    {
        get;
    }
    public IReadOnlyList<(string Role, int Lane, TilePosition Position, SpaceBeltPortSenderSimulation Sender)> Outputs
    {
        get;
    }
    internal BoundBeltPorts(List<(string, int, TilePosition, IItemReceiver)> inputs, List<(string, int, TilePosition, SpaceBeltPortSenderSimulation)> outputs)
    {
        Inputs = inputs;
        Outputs = outputs;
    }
}
