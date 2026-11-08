using Newtonsoft.Json;

public abstract class ExperimentResult
{
    [JsonProperty("verified")]
    public bool Verified
    {
        get;
    }
    private protected ExperimentResult(bool verified) => Verified = verified;
    public static ExperimentResult Accepted<T>(T details) => new Result<T>(true, details);
    public static ExperimentResult Observed<T>(T details) => new Result<T>(false, details);

    private sealed class Result<T> : ExperimentResult
    {
        [JsonProperty("details")]
        public T Details
        {
            get;
        }
        internal Result(bool verified, T details) : base(verified) => Details = details;
    }
}
