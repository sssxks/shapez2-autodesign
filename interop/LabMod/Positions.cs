using System;

public readonly struct TilePosition : IEquatable<TilePosition>
{
    private readonly (int X, int Y, int Z) value;
    public TilePosition(int x, int y, int z) => value = (x, y, z);
    public (int, int, int) ToTuple() => value;
    public bool Equals(TilePosition other) => value.Equals(other.value);
    public override bool Equals(object? obj) => obj is TilePosition other && Equals(other);
    public override int GetHashCode() => value.GetHashCode();
    public override string ToString() => value.ToString();
}

public readonly struct ChunkPosition : IEquatable<ChunkPosition>
{
    private readonly (int X, int Y, int Z) value;
    public ChunkPosition(int x, int y, int z) => value = (x, y, z);
    public bool Equals(ChunkPosition other) => value.Equals(other.value);
    public override bool Equals(object? obj) => obj is ChunkPosition other && Equals(other);
    public override int GetHashCode() => value.GetHashCode();
    public override string ToString() => value.ToString();
}
