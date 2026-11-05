using System.Text.Json;
using Mono.Cecil;

if (args.Length != 1) throw new ArgumentException("HostPrep <staged-game-directory>");
var root = Path.GetFullPath(args[0]);
if (!File.Exists(Path.Combine(root, ".shapez-lab-staged")))
    throw new InvalidOperationException("Only a marked lab clone may be patched");

var managed = Path.Combine(root, "shapez 2_Data", "Managed");
var auditPath = Path.Combine(root, "patch-audit.json");
// A failed preparation must not leave an old success marker usable by the host.
File.Delete(auditPath);
using var resolver = new DefaultAssemblyResolver();
resolver.AddSearchDirectory(managed);
var patches = new List<AssemblyPatch>();
foreach (var file in Directory.EnumerateFiles(managed, "*.dll").Order(StringComparer.Ordinal))
{
    var name = Path.GetFileName(file);
    if (!name.StartsWith("Game.", StringComparison.Ordinal) &&
        !name.StartsWith("SPZ", StringComparison.Ordinal) && name != "MainMenuAssembly.dll")
        continue;
    using var input = new MemoryStream(File.ReadAllBytes(file));
    using var assembly = AssemblyDefinition.ReadAssembly(input, new ReaderParameters { AssemblyResolver = resolver });
    var patch = AssemblyPatch.Create(file, assembly);
    if (patch != null)
        patches.Add(patch);
}

AssemblyPatch.Validate(patches); // Discover and validate every target before writing any DLL.
foreach (var patch in patches) patch.Write();
var audit = patches.Select(patch => patch.Audit).ToArray();
var json = JsonSerializer.Serialize(audit, new JsonSerializerOptions { WriteIndented = true });
File.WriteAllText(auditPath, json);
Console.WriteLine(json);
