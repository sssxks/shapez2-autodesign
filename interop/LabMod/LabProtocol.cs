using System;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

internal sealed class LabRequest
{
    internal ulong Id
    {
        get;
    }
    internal string Operation { get; }
    internal LabArguments Arguments { get; }

    internal LabRequest(ulong id, JObject payload)
    {
        Id = id;
        Operation = JsonInput.String(payload, "operation");
        Arguments = new LabArguments(JsonInput.Object(JsonInput.Required(payload, "args"), "args"));
    }
}

internal sealed class LabResponse
{
    [JsonProperty("id")]
    public ulong Id
    {
        get;
    }
    [JsonProperty("ok")]
    public bool Ok
    {
        get;
    }
    [JsonProperty("wall_seconds")]
    public double WallSeconds
    {
        get;
    }
    [JsonProperty("result", NullValueHandling = NullValueHandling.Ignore)]
    public object? Result
    {
        get;
    }
    [JsonProperty("error", NullValueHandling = NullValueHandling.Ignore)]
    public string? Error
    {
        get;
    }

    internal LabResponse(ulong id, double seconds, object? result, string? error)
    {
        Id = id;
        WallSeconds = seconds;
        Result = result;
        Error = error;
        Ok = error == null;
    }
}

internal sealed class LabReply
{
    internal object Value
    {
        get;
    }
    internal bool Shutdown
    {
        get;
    }
    internal LabReply(object value, bool shutdown = false)
    {
        Value = value;
        Shutdown = shutdown;
    }
}
