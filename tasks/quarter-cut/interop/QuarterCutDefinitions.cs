using System.Linq;
using Game.Core.Coordinates;

/// Query only the resolved transport capabilities used by the quarter-cut task.
public sealed class QuarterCutDefinitions : IBlueprintExperiment
{
    public void Configure(LabArguments args)
    {
    }
    public bool KeepBuilding(GlobalTileCoordinate position) => true;
    public ExperimentResult Run(LabWorld world, LabArguments args, string root) =>
        ExperimentResult.Observed(BuildingCatalog.Query(id => !id.StartsWith("Virtual") &&
            new[] { "Cutter", "Rotator", "Splitter", "Merger", "Belt", "Lift", "Stacker" }.Any(id.Contains)));
}
