using System;
using Game.Core.Blueprint;
using Core;
using Game.Core.Coordinates;
using Game.Orchestration;

/// Scoped native player/session context for disposable acceptance worlds.
internal sealed class LabPlayerActions : IDisposable
{
    // Native action API depends on this deprecated session interface.
#pragma warning disable CS0618
    private readonly IGameSessionManagers previous;
#pragma warning restore CS0618
    private readonly HubObserver hub;
    internal Player Player
    {
        get;
    }
    internal BaseMapInteractionMode Interaction
    {
        get;
    }

    internal LabPlayerActions(LabWorld world)
    {
        var session = (GameSessionOrchestrator)GameBindings.Session;
        var mode = GameBindings.Mode;
        var research = GameBindings.Field<ResearchManager>(session, "Research");
        foreach (var upgrade in research.Layout.AllUpgrades)
            research.UnlockManager.TryUnlock(upgrade, forced: true);
        research.BlueprintCurrencyManager.AddBlueprintCurrency(new BlueprintCurrency(10000000L, 0));
        Player = new Player(PlayerRole.LocalHost) { CurrentMap = world.Model };
        Interaction = GameBindings.Field<BaseMapInteractionMode>(session, "InteractionMode");
        hub = new HubObserver(new[] { mode.Islands.Hub.Id }, world.Model, research.LinearUpgradeManager,
            mode.ResearchLayout.LinearUpgrades.HubInputSize, mode.MaxBuildingLayer);
        // These native actions still use the game's obsolete static session dependency.
#pragma warning disable CS0618
        previous = StaticGameCoreAccessor.G;
        StaticGameCoreAccessor.AssignGameSessionManagers(new GameSessionManagers(Player,
            GameBindings.Field<Savegame>(session, "Savegame"), mode, null!, GameBindings.Shapes,
            GameBindings.Fluids, null!, research, null!, null!, null!, Interaction, hub,
            session.DataSerializers, null!));
#pragma warning restore CS0618
        // Bootstrap the real hub observer away from the factory, then remove its fixture island.
        mode.Islands.Hub.TryCreateConfiguration(out var configuration);
        using var bootstrap = new Player(PlayerRole.GameInternal) { CurrentMap = world.Model };
        var action = new ActionModifyIsland(world.Model, bootstrap, new ActionModifyIsland.Payload(new[] {
            new ActionModifyIsland.PlacePayload(mode.Islands.Hub, configuration,
                new GlobalChunkCoordinate(-10000,-10000,0), default(GridRotation))
        }, ignorePlacementBlueprintCost: true));
        if (!action.TryExecute_INTERNAL(out var undo, Interaction))
            throw new InvalidOperationException("Cannot initialize placement hub fixture");
        if (!undo.TryExecute_INTERNAL(out _, Interaction))
            throw new InvalidOperationException("Cannot remove placement hub fixture");
    }

    public void Dispose()
    {
#pragma warning disable CS0618
        StaticGameCoreAccessor.AssignGameSessionManagers(previous);
#pragma warning restore CS0618
        hub.Dispose();
        Player.Dispose();
    }
}

