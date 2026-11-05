using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Core.Logging;
using Game.Blueprints;
using Game.Core.Blueprint;
using Game.Core.Blueprint.Importer.Deserialization;
using Game.Core.Blueprint.Importer.Disassemble;
using Game.Core.Blueprint.Serialization;
using Game.Core.Blueprints;
using Game.Orchestration;
using Newtonsoft.Json.Linq;

/// Inspect serialization and library compatibility without placing or saving a world.
internal static class BlueprintInspection
{
    internal static object Read(JObject args, string root)
    {
        var path = JsonInput.String(args, "blueprint");
        var name = JsonInput.CaseName(args, "blueprint");
        var expectedFormat = JsonInput.Integer(args, "expected_format_version");
        var expectedWarning = JsonInput.Boolean(args, "expected_library_warning");
        var expectedIcon = args["expected_icon"] as JArray;
        var text = File.ReadAllText(path);
        var logger = GameBindings.Field<ILogger>(GameBindings.Session, "Logger");
        var migrators = new BlueprintMigrators(logger);
        var disassembler = new BlueprintDisassembler(migrators.FormatMigrators);
        if (!disassembler.TryDisassembleBlueprint(text, out var content, out var metadata, out var formatVersion, out var error))
            throw error;
        var converter = new CustomDataConverter(new List<Game.Blueprints.IBlueprintAdditionalDataConverter> { new BlueprintIconConverter() });
        // Importer normally tolerates failed metadata and substitutes an empty annotation.
        // Inspect it explicitly so that an invalid metadata envelope fails this operation.
        var deserializer = new BlueprintDeserializer(new GZipBlueprintDecompressor(), converter);
        if ((formatVersion >= 5 || !string.IsNullOrEmpty(metadata)) &&
            !deserializer.TryDeserializeMetadata(metadata, out var annotation, out error))
            throw error;
        if (!GameBindings.Importer.TryImport(text, out var blueprint, out var importedVersion, out error))
            throw error;
        var entry = new BlueprintLibraryEntry("inspection", path, blueprint, importedVersion);
        var warning = entry.Version < BlueprintFormat.RecommendedVersion;
        if (expectedFormat != null && formatVersion != expectedFormat)
            throw new InvalidDataException("Unexpected blueprint format: " + formatVersion);
        if (expectedWarning != null && warning != expectedWarning)
            throw new InvalidDataException("Unexpected library warning: " + warning);
        var exporter = new BlueprintImporterFactory(converter).CreateExporter();
        var exported = exporter.Export(blueprint);
        if (!GameBindings.Importer.TryImport(exported, out var roundtrip, out var exportedVersion, out error))
            throw error;
        string Data(string encoded)
        {
            if (!disassembler.TryDisassembleBlueprint(encoded, out var data, out var extra, out var version, out var failure))
                throw failure;
            if (!new GZipBlueprintDecompressor().TryDecompress(data, out failure, out var decoded))
                throw failure;
            return decoded;
        }
        // Native gzip headers contain a timestamp; compare the uncompressed canonical payload.
        if (Data(exported) != Data(exporter.Export(roundtrip)))
            throw new InvalidDataException("Native blueprint roundtrip changed serialized data");
        if (!blueprint.GetBlueprintIcon().Equals(roundtrip.GetBlueprintIcon()))
            throw new InvalidDataException("Native blueprint roundtrip changed icon");
        var output = Path.Combine(root, name + "-native.spz2bp");
        File.WriteAllText(output, exported);
        var icon = entry.BlueprintIcon.Serialize().Data;
        if (expectedIcon != null && !icon.SequenceEqual(expectedIcon.Select(x => (string?)x)))
            throw new InvalidDataException("Unexpected blueprint icon");
        return new
        {
            verified = true,
            format_version = formatVersion,
            library_version = entry.Version,
            library_warning = warning,
            buildings = blueprint.Blueprint is IslandBlueprint islands ? islands.BuildingCount : ((BuildingBlueprint)blueprint.Blueprint).BuildingCount,
            icon,
            native_export_format = exportedVersion,
            native_export = output
        };
    }
}
