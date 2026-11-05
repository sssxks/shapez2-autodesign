using System;
using System.Collections.Generic;
using System.Linq;
using Game.Core.Map.Simulation;
using Game.Core.Trains;
using Game.Content.Features.Fluids;
using Newtonsoft.Json.Linq;

public sealed class UnloaderSupply
{
    readonly List<Action> refill = new List<Action>();
    public UnloaderSupply(ISimulator simulator, JObject manifest) : this(simulator, new SupplyManifest(manifest)) { }

    public UnloaderSupply(ISimulator simulator, SupplyManifest manifest)
    {
        var ids = GameBindings.ShapeIds;
        var registry = GameBindings.Fluids;
        var colors = GameBindings.Colors;
        var sources = manifest.Sources;
        int found = 0;
        foreach (var localized in simulator.Simulations)
        {
            var sim = localized.Simulation;
            var p0 = localized.GetOccupiedChunk(0);
            if ((sim is TrainCargoUnloaderSimulation<ShapeId> || sim is TrainCargoUnloaderSimulation<FluidId>) &&
                sources.TryGetValue(new ChunkPosition(p0.x, p0.y, p0.z), out var item))
            {
                found++;
                if (sim is TrainCargoUnloaderSimulation<ShapeId> shape)
                {
                    var id = ids.Resolve(item);
                    foreach (var f in GameBindings.ShapeCargo(shape))
                        refill.Add(() => { if (f.Package.Amount < 1000) f.LoadPackage(new CargoPackage<ShapeId>(30000, id)); });
                }
                else if (sim is TrainCargoUnloaderSimulation<FluidId> fluid)
                {
                    var id = registry.GetFluidId(colors.Single(f => f.Color.Code == item[0]));
                    foreach (var f in GameBindings.FluidCargo(fluid))
                        refill.Add(() => { if (f.Package.Amount < 1000) f.LoadPackage(new CargoPackage<FluidId>(30000, id)); });
                }
            }
        }
        if (found != sources.Count)
            throw new Exception($"Unloader mismatch: {found}/{sources.Count}");
    }
    public void BeforeTick()
    {
        foreach (var f in refill)
            f();
    }
}

