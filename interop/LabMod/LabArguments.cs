using System;
using System.IO;
using System.Linq;
using Newtonsoft.Json.Linq;

// Decode the wire format once; operations and plugins receive typed values only.
public sealed class LabArguments
{
    public string? Name
    {
        get;
    }
    public string? Blueprint
    {
        get;
    }
    public string? Target
    {
        get;
    }
    public int? Seconds
    {
        get;
    }
    public int? WarmupSeconds
    {
        get;
    }
    public int? ExpectedBuildings
    {
        get;
    }
    public int? ExpectedMergers
    {
        get;
    }
    public int? ExpectedFormatVersion
    {
        get;
    }
    public int? OutputRow
    {
        get;
    }
    public long? MinProduced
    {
        get;
    }
    public bool? ExportGraph
    {
        get;
    }
    public bool? ExpectedLibraryWarning
    {
        get;
    }
    public bool? Mirrored
    {
        get;
    }
    public bool? JumpInput
    {
        get;
    }
    public bool? AdapterProbe
    {
        get;
    }
    public double? MinimumRateRatio
    {
        get;
    }
    public string?[]? ExpectedIcon
    {
        get;
    }
    public InputCoordinate[]? Inputs
    {
        get;
    }
    public InputCoordinate[]? Outputs
    {
        get;
    }
    public double[]? InputRates
    {
        get;
    }
    public bool[]? OutputFull
    {
        get;
    }
    public PortInput[]? Ports
    {
        get;
    }
    public ManifestInput? RateManifest
    {
        get;
    }
    public ManifestInput? ProductionManifest
    {
        get;
    }
    public RecipeInput? Recipe
    {
        get;
    }
    internal ExperimentDefinition? Experiment
    {
        get;
    }
    internal LabArguments(JObject input)
    {
        Name = JsonInput.OptionalString(input, "name");
        Blueprint = JsonInput.OptionalString(input, "blueprint");
        Target = JsonInput.OptionalString(input, "target");
        Seconds = JsonInput.Integer(input, "seconds");
        WarmupSeconds = JsonInput.Integer(input, "warmup_seconds");
        ExpectedBuildings = JsonInput.Integer(input, "expected_buildings");
        ExpectedMergers = JsonInput.Integer(input, "expected_mergers");
        ExpectedFormatVersion = JsonInput.Integer(input, "expected_format_version");
        OutputRow = JsonInput.Integer(input, "output_row");
        MinProduced = JsonInput.Long(input, "min_produced");
        ExportGraph = JsonInput.Boolean(input, "export_graph");
        ExpectedLibraryWarning = JsonInput.Boolean(input, "expected_library_warning");
        Mirrored = JsonInput.Boolean(input, "mirrored");
        JumpInput = JsonInput.Boolean(input, "jump_input");
        AdapterProbe = JsonInput.Boolean(input, "adapter_probe");
        MinimumRateRatio = JsonInput.Number(input, "minimum_rate_ratio");
        ExpectedIcon = ReadArray(input, "expected_icon", v => v.Type == JTokenType.Null ? null :
            v.Type == JTokenType.String ? (string?)v : throw new ArgumentException("expected_icon requires strings or null"));
        Inputs = ReadArray(input, "inputs", v => new InputCoordinate(v));
        Outputs = ReadArray(input, "outputs", v => new InputCoordinate(v));
        InputRates = ReadArray(input, "input_rates", v =>
        {
            if (v.Type != JTokenType.Float && v.Type != JTokenType.Integer)
                throw new ArgumentException("input_rates requires numbers");
            var rate = (double)v;
            if (double.IsNaN(rate) || double.IsInfinity(rate) || rate < 0)
                throw new ArgumentException("input_rates requires finite nonnegative numbers");
            return rate;
        });
        OutputFull = ReadArray(input, "output_full", v => v.Type == JTokenType.Boolean ? (bool)v : throw new ArgumentException("output_full requires booleans"));
        Ports = ReadArray(input, "ports", v => new PortInput(JsonInput.Object(v, "port")));
        RateManifest = ReadObject(input, "rate_manifest", v => new ManifestInput(v));
        ProductionManifest = ReadObject(input, "production_manifest", v => new ManifestInput(v));
        Recipe = ReadObject(input, "recipe", v => new RecipeInput(v));
        Experiment = ReadObject(input, "experiment", v => new ExperimentDefinition(v));
    }
    internal static T[]? ReadArray<T>(JObject input, string name, Func<JToken, T> read) =>
        input[name] == null || input[name]!.Type == JTokenType.Null ? null : JsonInput.Array(input, name).Select(read).ToArray();
    private static T? ReadObject<T>(JObject input, string name, Func<JObject, T> read) where T : class =>
        input[name] == null || input[name]!.Type == JTokenType.Null ? null : read(JsonInput.Object(input[name]!, name));
}
public readonly struct InputCoordinate
{
    public int X
    {
        get;
    }
    public int Y
    {
        get;
    }
    public int Z
    {
        get;
    }
    internal InputCoordinate(JToken value)
    {
        (X, Y, Z) = JsonInput.Coordinate(value, "position");
    }
    public override string ToString() => $"({X}, {Y}, {Z})";
}
public sealed class PortInput
{
    public string Role
    {
        get;
    }
    public int Lane
    {
        get;
    }
    public InputCoordinate Position
    {
        get;
    }
    public bool Output
    {
        get;
    }
    internal PortInput(JObject input)
    {
        Role = JsonInput.String(input, "role");
        Lane = JsonInput.Integer(input, "lane") ?? throw new ArgumentException("lane required");
        Position = new InputCoordinate(JsonInput.Required(input, "position"));
        Output = JsonInput.Boolean(input, "output") ?? throw new ArgumentException("output required");
    }
}
public sealed class ItemPortInput
{
    public InputCoordinate Position
    {
        get;
    }
    public string Item
    {
        get;
    }
    internal ItemPortInput(JObject input, string position)
    {
        Position = new InputCoordinate(JsonInput.Required(input, position));
        Item = JsonInput.String(input, "item");
    }
}
public sealed class ManifestInput
{
    public ItemPortInput[]? Sources
    {
        get;
    }
    public ItemPortInput[]? Targets
    {
        get;
    }
    public InputCoordinate[]? CutIslands
    {
        get;
    }
    public ProductionInput? Production
    {
        get;
    }
    internal ManifestInput(JObject input)
    {
        Sources = LabArguments.ReadArray(input, "sources", v => new ItemPortInput(JsonInput.Object(v, "source"), "chunk"));
        Targets = LabArguments.ReadArray(input, "targets", v => new ItemPortInput(JsonInput.Object(v, "target"), "tile"));
        CutIslands = LabArguments.ReadArray(input, "cut_islands", v => new InputCoordinate(v));
        Production = input["production"] == null ? null : new ProductionInput(JsonInput.Object(input["production"]!, "production"));
    }
}
public sealed class ProductionInput
{
    public InputCoordinate OutputChunk
    {
        get;
    }
    public int SignalReceivers
    {
        get;
    }
    internal ProductionInput(JObject input)
    {
        OutputChunk = new InputCoordinate(JsonInput.Required(input, "output_chunk"));
        SignalReceivers = JsonInput.Integer(input, "signal_receivers") ?? throw new ArgumentException("signal_receivers is required");
    }
}
public sealed class RecipeInput
{
    public string Target
    {
        get;
    }
    public int Root
    {
        get;
    }
    public RecipeNodeInput[] Nodes
    {
        get;
    }
    internal RecipeInput(JObject input)
    {
        Target = JsonInput.String(input, "target");
        Root = JsonInput.Integer(input, "root") ?? throw new ArgumentException("Recipe root required");
        Nodes = JsonInput.Array(input, "nodes").Select(v => new RecipeNodeInput(JsonInput.Object(v, "node"))).ToArray();
    }
}
public sealed class RecipeNodeInput
{
    public int Id
    {
        get;
    }
    public string Op
    {
        get;
    }
    public int[] Inputs
    {
        get;
    }
    public string Expected
    {
        get;
    }
    public string? Color
    {
        get;
    }
    internal RecipeNodeInput(JObject input)
    {
        Id = JsonInput.Integer(input, "id") ?? throw new ArgumentException("Recipe node id required");
        Op = JsonInput.String(input, "op");
        Inputs = JsonInput.Array(input, "inputs").Select(v => v.Type == JTokenType.Integer ? (int)v : throw new ArgumentException("Recipe input must be an integer")).ToArray();
        Expected = JsonInput.String(input, "expected");
        Color = JsonInput.OptionalString(input, "color");
    }
}
public static class ArgumentChecks
{
    public static T Required<T>(T? value, string name) where T : class => value ?? throw new ArgumentException(name + " is required");
    public static int Nonnegative(int value, string name) => value >= 0 ? value : throw new ArgumentOutOfRangeException(name, "Must be nonnegative");
    public static string CaseName(LabArguments args, string fallback)
    {
        var name = args.Name ?? fallback;
        if (name == "." || name == ".." || name.IndexOfAny(Path.GetInvalidFileNameChars()) >= 0)
            throw new ArgumentException("Invalid case name");
        return name;
    }
}

internal sealed class ExperimentDefinition
{
    internal string Assembly
    {
        get;
    }
    internal string Type
    {
        get;
    }
    internal ExperimentDefinition(JObject input)
    {
        Assembly = JsonInput.String(input, "assembly");
        Type = JsonInput.String(input, "type");
    }
}
