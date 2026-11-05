using System;
using System.IO;
using System.Collections.Generic;
using Game.Core.Map.Simulation;
using Game.Core.Simulation;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

internal static class GraphExporter
{
    internal static (int count, List<object> open, string path) Write(Simulator simulator, JObject args, string root)
    {
        var nodes = new List<object>();
        var open = new List<object>();
        foreach (var sim in ((ISimulator)simulator).Simulations)
        {
            var positions = new List<int[]>();
            if (sim is ILocalizedTileSimulation tile)
            for (int i = 0; i < tile.NumOccupiedTiles; i++)
            {
                var p = tile.GetOccupiedTile(i);
                positions.Add(new[] { p.x, p.y, (int)p.z });
            }
            var links = new List<object>();
            if (sim is IConnectableSimulation connectable)
            for (int i = 0; i < connectable.NumConnectors; i++)
            {
                var connector = connectable.GetConnector(i);
                bool connected = simulator.TryGetConnectedSimulation(sim, i, out var other);
                links.Add(new
                {
                    port = i,
                    type = connector.GetType().FullName,
                    socket = connector.ConnectorSocket.ToString(),
                    target = connected ? Id(other) : null
                });
                if (!connected)
                    open.Add(new
                    {
                        node = Id(sim),
                        port = i,
                        type = connector.GetType().FullName,
                        socket = connector.ConnectorSocket.ToString()
                    });
            }
            nodes.Add(new
            {
                id = Id(sim),
                type = sim.Simulation.GetType().FullName,
                positions,
                links
            });
        }
        var name = ((string)args["name"] ?? "blueprint");
        if (name.IndexOfAny(Path.GetInvalidFileNameChars()) >= 0)
            throw new ArgumentException("Invalid case name");
        var graphPath = Path.Combine(root, name + "-graph.json");
        File.WriteAllText(graphPath, JsonConvert.SerializeObject(nodes));
        return (nodes.Count, open, graphPath);
    }
    static string Id(ILocalizedSimulation sim)
    {
        if (sim is ILocalizedTileSimulation t)
        {
            var p = t.GetOccupiedTile(0);
            return p.x + ":" + p.y + ":" + p.z + ":" + sim.Simulation.GetType().Name;
        }
        var c = sim.GetOccupiedChunk(0);
        return "chunk:" + c.x + ":" + c.y + ":" + c.z + ":" + sim.Simulation.GetType().Name;
    }
}
