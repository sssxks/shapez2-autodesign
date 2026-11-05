using System;
using System.Reflection;
using Game.Orchestration;
using System.Collections.Generic;
using System.Linq;
using Game.Core.Trains;
using Game.Content.Features.Fluids;

public static class GameBindings
{
    public static object Session
    {
        get
        {
            if (LabBootstrap.Session != null)
                return LabBootstrap.Session;
            var orchestrator = typeof(GameBootstrapper).GetField("GameOrchestrator", BindingFlags.Static | BindingFlags.NonPublic).GetValue(null);
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
    public static TrainCargoToBeltFillingContainer<ShapeId>[] ShapeCargo(object unloader) => Field<TrainCargoToBeltFillingContainer<ShapeId>[]>(unloader, "_FillingContainers");
    public static TrainCargoToBeltFillingContainer<FluidId>[] FluidCargo(object unloader) => Field<TrainCargoToBeltFillingContainer<FluidId>[]>(unloader, "_FillingContainers");
    public static T Field<T>(object obj, string name)
    {
        if (obj == null)
            throw new ArgumentNullException(nameof(obj));
        var field = obj.GetType().GetField(name, BindingFlags.NonPublic | BindingFlags.Public | BindingFlags.Instance);
        if (field == null)
            throw new MissingFieldException(obj.GetType().FullName, name);
        return (T)field.GetValue(obj);
    }
}
