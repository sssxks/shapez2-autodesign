using System;
using System.Collections.Concurrent;
using System.IO;
using System.IO.Pipes;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

// Only the worker touches the pipe. Unity's main thread consumes requests and
// supplies replies without waiting on I/O or calling game APIs on the worker.
internal sealed class LabPipe : IDisposable
{
    private const int MaxFrame = 64 * 1024 * 1024;
    private readonly NamedPipeServerStream pipe;
    private readonly CancellationTokenSource stop = new CancellationTokenSource();
    private readonly BlockingCollection<JObject> requests = new BlockingCollection<JObject>(1);
    private readonly BlockingCollection<object> replies = new BlockingCollection<object>(1);
    internal volatile bool QuitRequested;
    internal Exception Failure
    {
        get; private set;
    }

    internal LabPipe(string name)
    {
        pipe = new NamedPipeServerStream(name, PipeDirection.InOut, 1, PipeTransmissionMode.Byte);
        Task.Run((Action)Pump);
    }

    internal bool TryReceive(out JObject request) => requests.TryTake(out request);
    internal void Reply(object response) => replies.Add(response, stop.Token);

    private void Pump()
    {
        try
        {
            pipe.WaitForConnection();
            Write(replies.Take(stop.Token)); // Main-thread readiness after initialization.
            while (!stop.IsCancellationRequested)
            {
                var request = Read();
                requests.Add(request, stop.Token);
                Write(replies.Take(stop.Token));
                if ((string)request["operation"] == "shutdown")
                    break;
            }
        }
        catch (OperationCanceledException)
        {
        }
        catch (Exception error)
        {
            if (!stop.IsCancellationRequested)
                Failure = error;
        }
        finally
        {
            QuitRequested = true;
        }
    }

    private JObject Read()
    {
        var header = ReadExactly(4);
        uint size = (uint)header[0] | ((uint)header[1] << 8) | ((uint)header[2] << 16) | ((uint)header[3] << 24);
        if (size == 0 || size > MaxFrame)
            throw new InvalidDataException("Invalid pipe frame size: " + size);
        return JObject.Parse(Encoding.UTF8.GetString(ReadExactly((int)size)));
    }

    private byte[] ReadExactly(int size)
    {
        var bytes = new byte[size];
        int offset = 0;
        while (offset < size)
        {
            int count = pipe.Read(bytes, offset, size - offset);
            if (count == 0)
                throw new EndOfStreamException("Lab controller disconnected");
            offset += count;
        }
        return bytes;
    }

    private void Write(object value)
    {
        var bytes = Encoding.UTF8.GetBytes(JsonConvert.SerializeObject(value));
        if (bytes.Length > MaxFrame)
            throw new InvalidDataException("Pipe frame too large");
        int size = bytes.Length;
        var header = new byte[] { (byte)size, (byte)(size >> 8), (byte)(size >> 16), (byte)(size >> 24) };
        pipe.Write(header, 0, header.Length);
        pipe.Write(bytes, 0, bytes.Length);
        pipe.Flush();
    }

    public void Dispose()
    {
        stop.Cancel();
        pipe.Dispose();
    }
}
