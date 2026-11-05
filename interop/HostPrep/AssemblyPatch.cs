using Mono.Cecil;
using Mono.Cecil.Cil;

internal sealed record PatchAudit(string assembly, int persistentPathCalls, int quickBootConstructors, int headlessMethods);

internal sealed class AssemblyPatch
{
    private readonly string path;
    private readonly byte[] bytes;
    private readonly int prepares;
    private readonly int ticks;
    internal PatchAudit Audit
    {
        get;
    }

    private AssemblyPatch(string path, byte[] bytes, int paths, int boot, int prepares, int ticks)
    {
        this.path = path;
        this.bytes = bytes;
        this.prepares = prepares;
        this.ticks = ticks;
        Audit = new PatchAudit(Path.GetFileName(path), paths, boot, prepares + ticks);
    }

    internal static AssemblyPatch? Create(string path, AssemblyDefinition assembly)
    {
        var paths = 0;
        var boot = 0;
        var prepares = 0;
        var ticks = 0;
        foreach (var type in Flatten(assembly.MainModule.Types))
        {
            foreach (var method in type.Methods.Where(method => method.HasBody))
            {
                if (type.FullName == "GameSessionOrchestrator" && method.Name == "Prepare")
                {
                    PatchPrepare(assembly.MainModule, method);
                    prepares++;
                }
                else if (type.FullName == "GameSessionOrchestrator" && method.Name == "Tick")
                {
                    if (method.IsStatic || method.ReturnType.FullName != "System.Void")
                        throw new InvalidOperationException("Unsupported session Tick signature: " + method.FullName);
                    method.Body = new MethodBody(method);
                    method.Body.GetILProcessor().Append(Instruction.Create(OpCodes.Ret));
                    ticks++;
                }
                else
                {
                    paths += PatchDataPaths(assembly.MainModule, method);
                    if (type.FullName == "Game.Orchestration.GameOrchestrator" && method.IsConstructor && method.Parameters.Count == 1)
                    {
                        var mode = method.Parameters[0].ParameterType.Resolve().Fields.Single(field => field.Name == "StartInGame").Constant;
                        var il = method.Body.GetILProcessor();
                        var first = method.Body.Instructions[0];
                        il.InsertBefore(first, Instruction.Create(OpCodes.Ldc_I4, Convert.ToInt32(mode)));
                        il.InsertBefore(first, Instruction.Create(OpCodes.Starg, method.Parameters[0]));
                        boot++;
                    }
                }
            }
        }
        if (paths + boot + prepares + ticks == 0)
            return null;
        using var output = new MemoryStream();
        assembly.Write(output);
        return new AssemblyPatch(path, output.ToArray(), paths, boot, prepares, ticks);
    }

    private static void PatchPrepare(ModuleDefinition module, MethodDefinition method)
    {
        if (method.IsStatic || method.Parameters.Count != 6 ||
            method.Parameters.Any(parameter => parameter.ParameterType.IsValueType || parameter.ParameterType.IsByReference) ||
            method.ReturnType.FullName != "Cysharp.Threading.Tasks.UniTask")
            throw new InvalidOperationException("Unsupported session Prepare signature: " + method.FullName);
        method.Body = new MethodBody(method);
        var il = method.Body.GetILProcessor();
        var reference = new AssemblyNameReference("ShapezLab", new Version(1, 0, 0, 0));
        module.AssemblyReferences.Add(reference);
        var bridge = new TypeReference("", "LabBootstrap", module, reference);
        var prepare = new MethodReference("Prepare", method.ReturnType, bridge) { HasThis = false };
        for (var i = 0; i < 7; i++)
            prepare.Parameters.Add(new ParameterDefinition(module.TypeSystem.Object));
        il.Append(Instruction.Create(OpCodes.Ldarg_0));
        foreach (var parameter in method.Parameters)
            il.Append(Instruction.Create(OpCodes.Ldarg, parameter));
        il.Append(Instruction.Create(OpCodes.Call, prepare));
        il.Append(Instruction.Create(OpCodes.Ret));
    }

    private static int PatchDataPaths(ModuleDefinition module, MethodDefinition method)
    {
        var count = 0;
        foreach (var instruction in method.Body.Instructions.ToArray())
        {
            if (instruction.Operand is not MethodReference target || target.Name != "get_persistentDataPath" ||
                !target.DeclaringType.FullName.StartsWith("UnityEngine.", StringComparison.Ordinal))
                continue;
            // Import against the game's corlib, not the host .NET runtime.
            var environment = new TypeReference("System", "Environment", module, module.TypeSystem.CoreLibrary);
            var get = new MethodReference("GetEnvironmentVariable", module.TypeSystem.String, environment) { HasThis = false };
            get.Parameters.Add(new ParameterDefinition(module.TypeSystem.String));
            instruction.OpCode = OpCodes.Ldstr;
            instruction.Operand = "SHAPEZ_LAB_DATA";
            method.Body.GetILProcessor().InsertAfter(instruction, Instruction.Create(OpCodes.Call, get));
            count++;
        }
        return count;
    }

    internal static void Validate(IReadOnlyList<AssemblyPatch> patches)
    {
        // Expected targets for the supported 1.2.0-rc3 installation.
        var expectedPaths = new Dictionary<string, int>(StringComparer.Ordinal)
        {
            ["Game.Core.dll"] = 1,
            ["Game.Hud.View.Model.dll"] = 2,
            ["Game.Orchestration.dll"] = 1,
            ["SPZPluginsAssembly.dll"] = 1
        };
        if (patches.Sum(patch => patch.prepares) != 1 || patches.Sum(patch => patch.ticks) != 1 ||
            patches.Sum(patch => patch.Audit.quickBootConstructors) != 1)
            throw new InvalidOperationException("Expected one session Prepare, one session Tick and one quick-boot constructor");
        foreach (var expected in expectedPaths)
        {
            if (patches.SingleOrDefault(patch => patch.Audit.assembly == expected.Key)?.Audit.persistentPathCalls != expected.Value)
                throw new InvalidOperationException($"Expected {expected.Value} data-path calls in {expected.Key}");
        }
        if (patches.Sum(patch => patch.Audit.persistentPathCalls) != expectedPaths.Values.Sum())
            throw new InvalidOperationException("Unexpected game data-path patch targets");
    }

    internal void Write()
    {
        var temporary = path + ".lab-patched";
        try
        {
            File.WriteAllBytes(temporary, bytes);
            File.Move(temporary, path, true);
        }
        finally { File.Delete(temporary); }
    }

    private static IEnumerable<TypeDefinition> Flatten(IEnumerable<TypeDefinition> types)
    {
        foreach (var type in types)
        {
            yield return type;
            foreach (var nested in Flatten(type.NestedTypes))
                yield return nested;
        }
    }
}
