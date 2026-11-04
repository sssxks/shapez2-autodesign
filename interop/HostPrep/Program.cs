using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using Mono.Cecil;
using Mono.Cecil.Cil;

if(args.Length!=1) throw new ArgumentException("HostPrep <staged-game-directory>");
var root=Path.GetFullPath(args[0]);
if(!File.Exists(Path.Combine(root,".shapez-lab-staged"))) throw new InvalidOperationException("Only a marked lab clone may be patched");
var managed=Path.Combine(root,"shapez 2_Data","Managed");
var resolver=new DefaultAssemblyResolver();resolver.AddSearchDirectory(managed);
var audit=new List<object>();
foreach(var file in Directory.GetFiles(managed,"*.dll")) {
 var name=Path.GetFileName(file);
 if(!(name.StartsWith("Game.")||name.StartsWith("SPZ")||name=="MainMenuAssembly.dll")) continue;
 using var input=new MemoryStream(File.ReadAllBytes(file));
 using var asm=AssemblyDefinition.ReadAssembly(input,new ReaderParameters{AssemblyResolver=resolver});
 int paths=0,boot=0;
 foreach(var type in Flatten(asm.MainModule.Types)) foreach(var method in type.Methods) {
  if(!method.HasBody) continue;
  foreach(var ins in method.Body.Instructions.ToArray()) {
   if(ins.Operand is MethodReference target && target.Name=="get_persistentDataPath" && target.DeclaringType.FullName.StartsWith("UnityEngine.")) {
    // Import against the game's corlib, not the host .NET runtime.
    var envType=new TypeReference("System","Environment",asm.MainModule,asm.MainModule.TypeSystem.CoreLibrary);
    var get=new MethodReference("GetEnvironmentVariable",asm.MainModule.TypeSystem.String,envType){HasThis=false};
    get.Parameters.Add(new ParameterDefinition(asm.MainModule.TypeSystem.String));
    ins.OpCode=OpCodes.Ldstr;ins.Operand="SHAPEZ_LAB_DATA";
    method.Body.GetILProcessor().InsertAfter(ins,Instruction.Create(OpCodes.Call,get));paths++;
   }
  }
  if(type.FullName=="Game.Orchestration.GameOrchestrator" && method.IsConstructor && method.Parameters.Count==1) {
   var mode=method.Parameters[0].ParameterType.Resolve().Fields.Single(f=>f.Name=="StartInGame").Constant;
   var il=method.Body.GetILProcessor();var first=method.Body.Instructions[0];
   il.InsertBefore(first,Instruction.Create(OpCodes.Ldc_I4,Convert.ToInt32(mode)));
   il.InsertBefore(first,Instruction.Create(OpCodes.Starg,method.Parameters[0]));boot++;
  }
 }
 if(paths+boot>0) {
  var temp=file+".lab-patched";asm.Write(temp);File.Move(temp,file,true);
  audit.Add(new{assembly=name,persistentPathCalls=paths,quickBootConstructors=boot});
 }
}
File.WriteAllText(Path.Combine(root,"patch-audit.json"),JsonSerializer.Serialize(audit,new JsonSerializerOptions{WriteIndented=true}));
Console.WriteLine(JsonSerializer.Serialize(audit));
static IEnumerable<TypeDefinition> Flatten(IEnumerable<TypeDefinition> types) {
 foreach(var t in types){yield return t;foreach(var n in Flatten(t.NestedTypes))yield return n;}
}
