using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Reflection;
using Core.Dependency;
using Core.Logging;
using Game.Blueprints;
using Game.Core;
using Game.Core.Blueprint;
using Game.Core.Blueprint.Importer;
using Game.Core.Coordinates;
using Game.Core.Map.Simulation;
using Game.Core.Map.Simulation.Clustering;
using Game.Core.Simulation;
using Game.Orchestration;
using Game.Content.Features.Signals.Channels;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

internal sealed class LabWorld:IDisposable {
 readonly MapLayout layout;readonly Map map;readonly MapModel model;readonly Simulator simulator;
 readonly SimulationGraph graph;readonly ISimulationSystem[] systems;readonly ProcessingUpdateStrategy strategy;
 readonly GameMode mode;readonly ILogger logger;readonly BlueprintRawDataConverter converter;readonly IBlueprintImporter importer;
 public static object Session {
  get{var orchestrator=typeof(GameBootstrapper).GetField("GameOrchestrator",BindingFlags.Static|BindingFlags.NonPublic).GetValue(null);
   var session=Field<object>(orchestrator,"CurrentSubOrchestrator");if(!(session is GameSessionOrchestrator))throw new InvalidOperationException("Game session is not ready");return session;}
 }
 internal static T Field<T>(object obj,string name)=>(T)obj.GetType().GetField(name,BindingFlags.NonPublic|BindingFlags.Public|BindingFlags.Instance).GetValue(obj);
 public LabWorld(){
  var session=Session;logger=Field<ILogger>(session,"Logger");mode=Field<GameMode>(session,"Mode");
  var deps=Field<DependencyContainer>(session,"DependencyContainer");
  var generator=new DefaultMapGenerator(mode.Seed,mode.MapGenerationParameters,mode.Scenario.StartingLocation,mode.ShapesConfiguration,mode.ShapeColorScheme,Field<ShapeRegistry>(session,"ShapeRegistry"),Field<IShapeIdManager>(session,"ShapeIdManager"));
  var resources=new GameResourcesMap(generator);
  // Construct a new system collection. Stateful systems must never be shared with the host map.
  var constructor=typeof(BuiltinSimulationSystems).GetConstructors().Single();
  var values=constructor.GetParameters().Select(p=>p.Name switch {
   "initialSimulationTime"=>(object)Ticks.Zero,"shapeOperations"=>Field<object>(session,"ShapeOperations"),
   "mode"=>mode,"shapeRegistry"=>Field<object>(session,"ShapeRegistry"),"shapeIdManager"=>Field<object>(session,"ShapeIdManager"),
   "resourcesMap"=>resources,"fluidRegistry"=>Field<object>(session,"FluidRegistry"),"packageItemSolver"=>Field<object>(session,"FluidPackageItemSolver"),
   "signalChannelRegistry"=>new SignalChannelRegistry(Enumerable.Empty<(SignalChannelId,IReadOnlySignalChannel)>()),"researchUnlockManager"=>Field<ResearchManager>(session,"Research").UnlockManager,
   "logger"=>logger,_=>throw new InvalidOperationException("Unknown dependency: "+p.Name)
  }).ToArray();
  systems=((BuiltinSimulationSystems)constructor.Invoke(values)).CreateSimulationSystems().ToArray();
  layout=new MapLayout(logger);map=new Map(layout,resources,logger);graph=new SimulationGraph(Ticks.Zero,false,logger);
  simulator=new Simulator(graph,systems,layout,logger);model=new MapModel(map,simulator,new MapLayers(mode.MaxBuildingLayer,mode.MinIslandLayer,mode.MaxIslandLayer),logger);
  strategy=new ProcessingUpdateStrategy(SimulationConstants.MaxSimulationUpdateDelta,SimulationConstants.MaxSimulatorIncrement,new[]{Ticks.Zero});
  importer=deps.Resolve<IBlueprintImporter>();
  converter=new BlueprintRawDataConverter(((GameSessionOrchestrator)session).DataSerializers,logger);
 }
 public object Run(JObject args,string root){
  var path=(string)args["blueprint"];if(string.IsNullOrEmpty(path))throw new ArgumentException("blueprint path required");
  if(!importer.TryImport(File.ReadAllText(path),out var annotated,out var version,out var error))throw error;
  if(!(annotated.Blueprint is IslandBlueprint blueprint))throw new ArgumentException("Island blueprint required");
  var originalCount=blueprint.BuildingCount;
  using(layout.StartBunchEdit()){
  foreach(var entry in blueprint.Entries){
   var transform=new GlobalChunkTransform(new GlobalChunkCoordinate(entry.Chunk_L.x,entry.Chunk_L.y,entry.Chunk_L.z),entry.Rotation);
   entry.Definition.TryCreateConfiguration(out var config);
   if(entry.Configuration!=null&&config!=null)converter.DeserializeByteArrayIntoData(entry.Configuration,GameVersionEnvironment.CurrentVersion,config.Sync);
   model.CreateIsland(entry.Definition,transform,config);
  }
  foreach(var entry in blueprint.Entries){
   var transform=new GlobalChunkTransform(new GlobalChunkCoordinate(entry.Chunk_L.x,entry.Chunk_L.y,entry.Chunk_L.z),entry.Rotation);
   if(entry.BuildingBlueprint==null)continue;
   foreach(var building in entry.BuildingBlueprint.Entries){
    building.Definition.TryCreateConfiguration(out var config);
    if(building.AdditionalConfigData!=null&&config!=null)converter.DeserializeByteArrayIntoData(building.AdditionalConfigData,GameVersionEnvironment.CurrentVersion,config.Sync);
    var t=new LocalTileTransform(building.Tile_L,building.Rotation).ToGlobal(new GlobalChunkTransform(transform.Position,default(GridRotation)));
    if((bool?)args["adapter_probe"]==true&&!AdapterProbe.KeepBuilding(t.Position))continue;
    model.CreateBuilding(building.Definition,t,config);
   }
  }
  }
  if(args["expected_buildings"]!=null && (int)args["expected_buildings"]!=model.BuildingCount)throw new Exception("Building count mismatch");
  var seconds=(int?)args["seconds"]??1;
  var lods=model.Islands.SelectMany(i=>i.Definition.Layout.GetChunkPositions().Select(c=>SimulationLOD.Highest(c.ToGlobal(i.Transform)))).ToArray();
  var update=new SimulationUpdateConfiguration(lods,TimeSpan.Zero,1);
  object probeReport=null;
  if((bool?)args["adapter_probe"]==true){
   var probe=new AdapterProbe(simulator);bool complete=false;
   for(int i=0;i<seconds*10;i++){probe.BeforeTick();simulator.SynchronousUpdate(Ticks.OneSecond/10,update,strategy);if(probe.AfterTick()){complete=true;break;}}
   probeReport=probe.Report();File.WriteAllText(Path.Combine(root,"adapter-flow.json"),JsonConvert.SerializeObject(probeReport));
   if(!complete)throw new Exception("Adapter flow did not reach every inlet: "+JsonConvert.SerializeObject(probeReport));
  }else for(int i=0;i<seconds*60;i++)simulator.SynchronousUpdate(Ticks.OneSecond/60,update,strategy);
  var nodes=new List<object>();var open=new List<object>();
  foreach(var sim in ((ISimulator)simulator).Simulations){
   var positions=new List<int[]>();if(sim is ILocalizedTileSimulation tile)for(int i=0;i<tile.NumOccupiedTiles;i++){var p=tile.GetOccupiedTile(i);positions.Add(new[]{p.x,p.y,(int)p.z});}
   var links=new List<object>();if(sim is IConnectableSimulation connectable)for(int i=0;i<connectable.NumConnectors;i++){
    var connector=connectable.GetConnector(i);bool connected=simulator.TryGetConnectedSimulation(sim,i,out var other);
    links.Add(new{port=i,type=connector.GetType().FullName,socket=connector.ConnectorSocket.ToString(),target=connected?Id(other):null});
    if(!connected)open.Add(new{node=Id(sim),port=i,type=connector.GetType().FullName,socket=connector.ConnectorSocket.ToString()});
   }
   nodes.Add(new{id=Id(sim),type=sim.Simulation.GetType().FullName,positions,links});
  }
  var name=((string)args["name"]??"blueprint");if(name.IndexOfAny(Path.GetInvalidFileNameChars())>=0)throw new ArgumentException("Invalid case name");
  var graphPath=Path.Combine(root,name+"-graph.json");File.WriteAllText(graphPath,JsonConvert.SerializeObject(nodes));
  return new{islands=model.IslandCount,buildings=model.BuildingCount,imported_buildings=originalCount,simulated_seconds=simulator.SimulationTime.FloatSeconds,simulations=nodes.Count,open_connectors=open,graph=graphPath,probe=probeReport};
 }
 static string Id(ILocalizedSimulation sim){if(sim is ILocalizedTileSimulation t){var p=t.GetOccupiedTile(0);return p.x+":"+p.y+":"+p.z+":"+sim.Simulation.GetType().Name;}var c=sim.GetOccupiedChunk(0);return "chunk:"+c.x+":"+c.y+":"+c.z+":"+sim.Simulation.GetType().Name;}
 public void Dispose(){model.Dispose();simulator.Dispose();foreach(var s in systems)if(s is IDisposable disposable)disposable.Dispose();graph.Dispose();map.Dispose();layout.Dispose();}
}

