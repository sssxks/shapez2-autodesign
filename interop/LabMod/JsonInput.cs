using System;
using System.IO;
using Newtonsoft.Json.Linq;

// JSON is parsed here or at an operation/plugin entry point, never in a tick loop.
public static class JsonInput
{
    public static JObject ReadFile(string path) => JObject.Parse(File.ReadAllText(path));

    public static JToken Required(JObject input, string name) =>
        input[name] is JToken value && value.Type != JTokenType.Null
            ? value : throw new ArgumentException($"{name} is required");

    public static string String(JObject input, string name) =>
        OptionalString(input, name) ?? throw new ArgumentException($"{name} is required");

    public static string? OptionalString(JObject input, string name)
    {
        var value = input[name];
        if (value == null || value.Type == JTokenType.Null)
            return null;
        if (value.Type != JTokenType.String || string.IsNullOrWhiteSpace((string?)value))
            throw new ArgumentException($"{name} must be a nonempty string");
        return (string?)value;
    }

    public static int? Integer(JObject input, string name) =>
        Scalar<int>(input, name, JTokenType.Integer);
    public static long? Long(JObject input, string name) =>
        Scalar<long>(input, name, JTokenType.Integer);
    public static bool? Boolean(JObject input, string name) =>
        Scalar<bool>(input, name, JTokenType.Boolean);

    public static double? Number(JObject input, string name)
    {
        var value = input[name];
        if (value == null || value.Type == JTokenType.Null)
            return null;
        if (value.Type != JTokenType.Float && value.Type != JTokenType.Integer)
            throw new ArgumentException($"{name} must be a number");
        var result = (double)value;
        if (double.IsNaN(result) || double.IsInfinity(result))
            throw new ArgumentException($"{name} must be finite");
        return result;
    }

    private static T? Scalar<T>(JObject input, string name, JTokenType kind) where T : struct
    {
        var value = input[name];
        if (value == null || value.Type == JTokenType.Null)
            return null;
        if (value.Type != kind)
            throw new ArgumentException($"{name} must be {kind}");
        return value.ToObject<T>();
    }

    public static JObject Object(JToken value, string name) =>
        value as JObject ?? throw new ArgumentException($"{name} must be an object");
    public static JArray Array(JObject input, string name) =>
        Required(input, name) as JArray ?? throw new ArgumentException($"{name} must be an array");

    public static int Nonnegative(int value, string name)
    {
        if (value < 0)
            throw new ArgumentOutOfRangeException(name, "Must be nonnegative");
        return value;
    }

    public static string CaseName(JObject input, string fallback)
    {
        var name = OptionalString(input, "name") ?? fallback;
        if (name == "." || name == ".." || name.IndexOfAny(Path.GetInvalidFileNameChars()) >= 0)
            throw new ArgumentException("Invalid case name");
        return name;
    }

    internal static (int X, int Y, int Z) Coordinate(JToken value, string name)
    {
        if (!(value is JArray items) || items.Count != 3)
            throw new ArgumentException($"{name} must contain three integer coordinates");
        foreach (var item in items)
            if (item.Type != JTokenType.Integer)
                throw new ArgumentException($"{name} coordinates must be integers");
        return ((int)items[0], (int)items[1], (int)items[2]);
    }
}
