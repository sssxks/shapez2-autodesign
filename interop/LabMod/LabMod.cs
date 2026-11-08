using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using Game.Content.BuildingPath.Simulation;
using Game.Core.Content;
using UnityEngine;

public sealed class ShapezLabMod : IMod
{
    private readonly GameObject? host;
    public ShapezLabMod()
    {
        if (string.IsNullOrEmpty(Environment.GetEnvironmentVariable("SHAPEZ_LAB_RUN")))
            return;
        host = new GameObject("ShapezLabHost");
        UnityEngine.Object.DontDestroyOnLoad(host);
        host.AddComponent<LabService>();
    }

    public void Dispose()
    {
        if (host != null)
            UnityEngine.Object.Destroy(host);
    }
}

public sealed class LabService : MonoBehaviour
{
    private string root = string.Empty;
    private bool ready;
    private LabPipe? pipe;

    private void Awake()
    {
        try
        {
            root = Environment.GetEnvironmentVariable("SHAPEZ_LAB_RUN")
                ?? throw new InvalidOperationException("SHAPEZ_LAB_RUN is required");
            var name = Environment.GetEnvironmentVariable("SHAPEZ_LAB_PIPE")
                ?? throw new InvalidOperationException("SHAPEZ_LAB_PIPE is required");
            Application.runInBackground = true;
            pipe = new LabPipe(name);
        }
        catch (Exception error) { Stop(error); }
    }

    private void OnDestroy() => pipe?.Dispose();

    private void Update()
    {
        var connection = pipe;
        if (connection == null)
            return;
        try
        {
            if (connection.QuitRequested)
            {
                Stop(connection.Failure);
                return;
            }
            if (!ready)
            {
                if (LabBootstrap.Session == null)
                    return;
                SendReady(connection);
                ready = true;
            }
            if (connection.TryReceive(out var request))
                ExecuteRequest(connection, request);
        }
        catch (Exception error) { Stop(error); }
    }

    private static void SendReady(LabPipe connection)
    {
        using var process = Process.GetCurrentProcess();
        connection.Reply(new
        {
            kind = "ready",
            protocol = 1,
            pid = process.Id,
            data_path = GameEnvironment.DataPath,
            version = Application.version,
            startup_seconds = (DateTime.Now - process.StartTime).TotalSeconds
        });
    }

    private void ExecuteRequest(LabPipe connection, LabRequest request)
    {
        var timer = Stopwatch.StartNew();
        LabResponse response;
        var shutdown = false;
        string operation = "request";
        try
        {
            operation = request.Operation;
            var result = Execute(operation, request.Arguments);
            shutdown = operation == "shutdown";
            response = new LabResponse(request.Id, timer.Elapsed.TotalSeconds, result, null);
        }
        catch (Exception error)
        {
            response = new LabResponse(request.Id, timer.Elapsed.TotalSeconds, null, $"{operation}: {error}");
        }
        connection.Reply(response, shutdown);
    }

    private void Stop(Exception? error)
    {
        if (error != null)
            UnityEngine.Debug.LogException(error);
        pipe?.Dispose();
        pipe = null;
        enabled = false;
        Application.Quit();
    }

    private object Execute(string operation, LabArguments args)
    {
        switch (operation)
        {
            case "shutdown":
                return new
                {
                    shutdown = true
                };
            case "recipe":
                return RecipeProbe.Run(args, root);
            case "inspect-save":
                return SaveInspection.Read(args);
            case "inspect-blueprint":
                return BlueprintInspection.Read(args, root);
            case "capacities":
                return ReadCapacities();
            case "capabilities":
                return new
                {
                    game = Application.version,
                    data = GameEnvironment.DataPath,
                    simulator = "Simulator",
                    headless = true,
                    scenario = GameBindings.Mode.Scenario.UniqueId.ToString(),
                    operations = new[] { "capabilities", "blueprint", "place-blueprint", "blueprint-diagnostic", "catalog", "capacities", "recipe", "inspect-save", "inspect-blueprint" }
                };
            case "blueprint":
                return BlueprintRunner.Run(args, root);
            case "blueprint-diagnostic":
                return BlueprintRunner.Run(args, root, diagnostic: true);
            case "place-blueprint":
                return BlueprintRunner.Run(args, root, placementOnly: true);
            case "catalog":
                return BuildingCatalog.Query();
            default:
                throw new ArgumentException("Unknown operation: " + operation);
        }
    }

    private static object ReadCapacities()
    {
        var mode = GameBindings.Mode;
        BeltSpeed belt = mode.Buildings.ForwardBelt.ConfigAs<IConveyorConfiguration>().ConveyorSpeed;
        BeltSpeed space = mode.Islands.SpaceBelts.First().ConfigAs<SpacePathConfiguration>().SpaceConveyorSpeed;
        BeltSpeed pipeSpeed = mode.Islands.SpacePipes.First().ConfigAs<SpacePathConfiguration>().SpaceConveyorSpeed;
        return new
        {
            belt = belt.ToString(),
            space = space.ToString(),
            pipe = pipeSpeed.ToString(),
            space_ratio = (double)space.StepsPerTick.Value / belt.StepsPerTick.Value,
            mode.MinIslandLayer,
            mode.MaxIslandLayer,
            space_ids = mode.Islands.SpaceBelts.Select(x => x.Id.ToString()).ToArray(),
            pipe_ids = mode.Islands.SpacePipes.Select(x => x.Id.ToString()).ToArray()
        };
    }

}
