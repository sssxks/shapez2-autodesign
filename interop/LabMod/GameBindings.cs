using System;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using System.Runtime.ExceptionServices;
using Core.Dependency;
using Core.Logging;
using Game.Content.Features.Fluids;
using Game.Content.Features.Signals.Channels;
using Game.Core;
using Game.Core.Blueprint.Importer;
using Game.Core.Map.Simulation;
using Game.Core.Simulation;
using Game.Core.Trains;
using Game.Orchestration;

public static class GameBindings
{
    public static object Session
    {
        get
        {
            if (LabBootstrap.Session != null)
                return LabBootstrap.Session;
            var orchestratorField = typeof(GameBootstrapper).GetField("GameOrchestrator", BindingFlags.Static | BindingFlags.NonPublic)
                ?? throw new MissingFieldException(typeof(GameBootstrapper).FullName, "GameOrchestrator");
            var orchestrator = orchestratorField.GetValue(null) ?? throw new InvalidOperationException("Game orchestrator is not ready");
            var session = Field<object>(orchestrator, "CurrentSubOrchestrator");
            if (!(session is GameSessionOrchestrator))
                throw new InvalidOperationException("Game session is not ready");
            return session;
        }
    }
    public static GameMode Mode => Field<GameMode>(Session, "Mode");
    public static ShapeRegistry Shapes => Field<ShapeRegistry>(Session, "ShapeRegistry");
    public static IShapeIdManager ShapeIds => Field<IShapeIdManager>(Session, "ShapeIdManager");
    public static ShapeOperations Operations => Field<ShapeOperations>(Session, "ShapeOperations");
    public static FluidRegistry Fluids => Field<FluidRegistry>(Session, "FluidRegistry");
    public static ColorFluid[] Colors => Field<Dictionary<IFluid, FluidId>>(Fluids, "FluidReferenceToIdLookup").Keys.OfType<ColorFluid>().ToArray();
    internal static IBlueprintImporter Importer => LabBootstrap.Importer ??
        Field<DependencyContainer>(Session, "DependencyContainer").Resolve<IBlueprintImporter>();

    internal static IEnumerable<ISimulationSystem> CreateSimulationSystems(GameResourcesMap resources, ILogger logger)
    {
        var session = Session;
        // Explicitly targets 1.2.0-rc3; constructor changes are compile errors.
        return new BuiltinSimulationSystems(Ticks.Zero, Operations, Mode, Shapes, ShapeIds,
            resources, Fluids, Field<FluidPackageItemSolver>(session, "FluidPackageItemSolver"),
            new SignalChannelRegistry(Enumerable.Empty<(SignalChannelId, IReadOnlySignalChannel)>()),
            Field<ResearchManager>(session, "Research").UnlockManager, logger).CreateSimulationSystems();
    }

    internal static void InitializeSession(object session, object options, object globals,
        object gameData, object platform, object content)
    {
        Invoke(session, "Init_1_GameOptions", globals, options, gameData, platform);
        Invoke(session, "Init_2_Player");
        Invoke(session, "Init_3_SavegameAndMode", content, gameData, options);
    }

    private static void Invoke(object target, string name, params object[] args)
    {
        var method = target.GetType().GetMethod(name, BindingFlags.Instance | BindingFlags.NonPublic)
            ?? throw new MissingMethodException(target.GetType().FullName, name);
        try
        {
            method.Invoke(target, args);
        }
        catch (TargetInvocationException error) when (error.InnerException != null)
        {
            ExceptionDispatchInfo.Capture(error.InnerException).Throw();
            throw;
        }
    }
    public static TrainCargoToBeltFillingContainer<ShapeId>[] ShapeCargo(object unloader) => Field<TrainCargoToBeltFillingContainer<ShapeId>[]>(unloader, "_FillingContainers");
    public static TrainCargoToBeltFillingContainer<FluidId>[] FluidCargo(object unloader) => Field<TrainCargoToBeltFillingContainer<FluidId>[]>(unloader, "_FillingContainers");
    public static T Field<T>(object obj, string name)
    {
        if (obj == null)
            throw new ArgumentNullException(nameof(obj));
        var field = obj.GetType().GetField(name, BindingFlags.NonPublic | BindingFlags.Public | BindingFlags.Instance);
        if (field == null)
            throw new MissingFieldException(obj.GetType().FullName, name);
        return field.GetValue(obj) is T value ? value
            : throw new InvalidOperationException($"{obj.GetType().FullName}.{name} must contain {typeof(T).FullName}");
    }
}
