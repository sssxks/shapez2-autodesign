using System;
using Game.Blueprints;
using Game.Orchestration;
using System.Collections.Generic;
using System.Reflection;
using Core.Logging;
using Cysharp.Threading.Tasks;
using Game.Core.Blueprint.Importer;
using Game.Core.Blueprint.Importer.Migration;
using Game.Core.Blueprint.Exporter;
using Game.Core.Blueprints;
using Game.Core.Blueprint;

// Called only by the patched temporary game clone. Retain native mode/content
// initialization; omit player interaction, audio, renderers, HUD and menu ticking.
public static class LabBootstrap
{
    internal static object? Session;
    internal static IBlueprintImporter? Importer;
    public static UniTask Prepare(object session, object options, object globals, object gameData, object platform, object content, object metadata)
    {
        var save = Environment.GetEnvironmentVariable("SHAPEZ_LAB_SAVE");
        if (!string.IsNullOrEmpty(save))
        {
            var copy = System.IO.Path.Combine(Environment.GetEnvironmentVariable("SHAPEZ_LAB_RUN"), "inspection.spz2");
            System.IO.File.Copy(save, copy);
            var saveLogger = GameBindings.Field<ILogger>(session, "Logger");
            var reader = new SaveFileAccessor(saveLogger).Read(copy, new Dictionary<Type, Game.Core.Serialization.IDataSerializer>());
            options = new GameStartOptionsContinueExisting(reader, false, "lab-inspection");
        }
        GameBindings.InitializeSession(session, options, globals, gameData, platform, content);
        var logger = GameBindings.Field<ILogger>(session, "Logger");
        var mode = GameBindings.Field<GameMode>(session, "Mode");
        var migrators = new BlueprintMigrators(logger);
        var factory = new BlueprintImporterFactory(new CustomDataConverter(new List<Game.Blueprints.IBlueprintAdditionalDataConverter> { new BlueprintIconConverter() }));
        Importer = factory.CreateImporter(mode.Buildings, mode.Islands, migrators.FormatMigrators, migrators.CandidateMigrators);
        foreach (var camera in UnityEngine.Resources.FindObjectsOfTypeAll<UnityEngine.Camera>())
            camera.enabled = false;
        UnityEngine.AudioListener.pause = true;
        UnityEngine.QualitySettings.vSyncCount = 0;
        Session = session;
        return UniTask.CompletedTask;
    }
}
