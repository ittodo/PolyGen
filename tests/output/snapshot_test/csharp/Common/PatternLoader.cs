// Pattern-based file loader for Polygen
// Supports glob patterns (*.csv), directory paths, and multi-file loading
using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;

namespace Polygen.Common
{
    /// <summary>
    /// Exception thrown when duplicate keys are detected during multi-file loading.
    /// </summary>
    public class DuplicateKeyException : Exception
    {
        public string Key { get; }
        public string FirstFile { get; }
        public string SecondFile { get; }

        public DuplicateKeyException(string key, string firstFile, string secondFile)
            : base($"Duplicate key '{key}' found in files: '{firstFile}' and '{secondFile}'")
        {
            Key = key;
            FirstFile = firstFile;
            SecondFile = secondFile;
        }
    }

    /// <summary>
    /// Result of loading a single file, tracking which file each row came from.
    /// </summary>
    public class LoadedFile<T>
    {
        public string FilePath { get; }
        public List<T> Rows { get; }

        public LoadedFile(string filePath, List<T> rows)
        {
            FilePath = filePath;
            Rows = rows;
        }
    }

    /// <summary>
    /// Utility class for pattern-based file loading.
    /// Supports:
    /// - Single file: "data/players.csv"
    /// - Glob pattern: "data/*.csv" or "data/players_*.csv"
    /// - Directory: "data/players/" (loads all matching files)
    /// </summary>
    public static class PatternLoader
    {
        /// <summary>
        /// Resolves a file pattern to a list of matching file paths, sorted alphabetically.
        /// </summary>
        /// <param name="rootDirectory">The root directory to resolve relative paths from.</param>
        /// <param name="pattern">The pattern to match (supports * wildcards).</param>
        /// <param name="extension">Default extension to use if pattern is a directory (e.g., ".csv").</param>
        /// <returns>List of matching file paths, sorted alphabetically.</returns>
        public static List<string> ResolvePattern(string rootDirectory, string pattern, string extension = ".csv")
        {
            if (string.IsNullOrEmpty(pattern))
                return new List<string>();

            // Normalize path separators
            pattern = pattern.Replace('/', Path.DirectorySeparatorChar).Replace('\\', Path.DirectorySeparatorChar);

            // Build full path
            string fullPattern = Path.IsPathRooted(pattern)
                ? pattern
                : Path.Combine(rootDirectory, pattern);

            var files = new List<string>();

            // Case 1: Directory path (ends with separator or is a directory)
            if (pattern.EndsWith(Path.DirectorySeparatorChar.ToString()) ||
                pattern.EndsWith("/") ||
                (Directory.Exists(fullPattern) && !File.Exists(fullPattern)))
            {
                string dir = fullPattern.TrimEnd(Path.DirectorySeparatorChar, '/');
                if (Directory.Exists(dir))
                {
                    files.AddRange(Directory.GetFiles(dir, "*" + extension));
                }
            }
            // Case 2: Glob pattern (contains * or ?)
            else if (pattern.Contains('*') || pattern.Contains('?'))
            {
                string? dir = Path.GetDirectoryName(fullPattern);
                string filePattern = Path.GetFileName(fullPattern);

                if (!string.IsNullOrEmpty(dir) && Directory.Exists(dir))
                {
                    files.AddRange(Directory.GetFiles(dir, filePattern));
                }
            }
            // Case 3: Single file
            else
            {
                if (File.Exists(fullPattern))
                {
                    files.Add(fullPattern);
                }
            }

            // Sort alphabetically for consistent ordering
            files.Sort(StringComparer.OrdinalIgnoreCase);
            return files;
        }

        /// <summary>
        /// Loads multiple files matching a pattern and merges them sequentially.
        /// </summary>
        /// <typeparam name="T">The type of rows to load.</typeparam>
        /// <typeparam name="TKey">The type of the primary key.</typeparam>
        /// <param name="rootDirectory">The root directory to resolve relative paths from.</param>
        /// <param name="pattern">The file pattern to match.</param>
        /// <param name="fileLoader">Function to load rows from a single file.</param>
        /// <param name="keySelector">Function to extract the primary key from a row.</param>
        /// <param name="extension">Default extension for directory patterns.</param>
        /// <returns>Merged list of all rows from all matching files.</returns>
        /// <exception cref="DuplicateKeyException">Thrown when duplicate keys are found across files.</exception>
        public static List<T> LoadAndMerge<T, TKey>(
            string rootDirectory,
            string pattern,
            Func<string, IEnumerable<T>> fileLoader,
            Func<T, TKey> keySelector,
            string extension = ".csv") where TKey : notnull
        {
            var files = ResolvePattern(rootDirectory, pattern, extension);
            var result = new List<T>();
            var keyToFile = new Dictionary<TKey, string>();

            foreach (var file in files)
            {
                var rows = fileLoader(file);
                foreach (var row in rows)
                {
                    var key = keySelector(row);
                    if (keyToFile.TryGetValue(key, out var existingFile))
                    {
                        throw new DuplicateKeyException(key?.ToString() ?? "(null)", existingFile, file);
                    }
                    keyToFile[key] = file;
                    result.Add(row);
                }
            }

            return result;
        }

        /// <summary>
        /// Loads multiple files matching a pattern and merges them sequentially.
        /// This overload does not check for duplicates (use when there's no primary key).
        /// </summary>
        public static List<T> LoadAndMergeAll<T>(
            string rootDirectory,
            string pattern,
            Func<string, IEnumerable<T>> fileLoader,
            string extension = ".csv")
        {
            var files = ResolvePattern(rootDirectory, pattern, extension);
            var result = new List<T>();

            foreach (var file in files)
            {
                result.AddRange(fileLoader(file));
            }

            return result;
        }

        /// <summary>
        /// Loads multiple files and returns them grouped by source file.
        /// Useful for debugging or when you need to know which file each row came from.
        /// </summary>
        public static List<LoadedFile<T>> LoadWithSources<T>(
            string rootDirectory,
            string pattern,
            Func<string, IEnumerable<T>> fileLoader,
            string extension = ".csv")
        {
            var files = ResolvePattern(rootDirectory, pattern, extension);
            var result = new List<LoadedFile<T>>();

            foreach (var file in files)
            {
                var rows = fileLoader(file).ToList();
                result.Add(new LoadedFile<T>(file, rows));
            }

            return result;
        }
    }
}
