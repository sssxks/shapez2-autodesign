using System;
using System.Collections.Generic;
using System.Linq;
using Game.Content.Features.Fluids;
using Game.Core.Map.Simulation;
using Game.Core.Trains;
using Newtonsoft.Json.Linq;

public sealed class UnloaderSupply
{
    private readonly List<(TrainCargoToBeltFillingContainer<ShapeId> Container, ShapeId Item)> shapes =
        new List<(TrainCargoToBeltFillingContainer<ShapeId>, ShapeId)>();
    private readonly List<(TrainCargoToBeltFillingContainer<FluidId> Container, FluidId Item)> fluids =
        new List<(TrainCargoToBeltFillingContainer<FluidId>, FluidId)>();
    private const int RefillThreshold = 1000;
    private const int PackageAmount = 30000;
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
                        shapes.Add((f, id));
                }
                else if (sim is TrainCargoUnloaderSimulation<FluidId> fluid)
                {
                    var id = registry.GetFluidId(colors.Single(f => f.Color.Code == item[0]));
                    foreach (var f in GameBindings.FluidCargo(fluid))
                        fluids.Add((f, id));
                }
            }
        }
        if (found != sources.Count)
            throw new Exception($"Unloader mismatch: {found}/{sources.Count}");
    }
    public void BeforeTick()
    {
        foreach (var source in shapes)
            if (source.Container.Package.Amount < RefillThreshold)
                source.Container.LoadPackage(new CargoPackage<ShapeId>(PackageAmount, source.Item));
        foreach (var source in fluids)
            if (source.Container.Package.Amount < RefillThreshold)
                source.Container.LoadPackage(new CargoPackage<FluidId>(PackageAmount, source.Item));
    }
}

