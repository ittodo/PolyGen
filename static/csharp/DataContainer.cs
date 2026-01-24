// This file is a part of the Polygen common utility library.
// It provides the container system for managing data tables and their relationships.
using System;
using System.Collections;
using System.Collections.Generic;
using System.Linq;

namespace Polygen.Common
{
    /// <summary>
    /// Base interface for all data rows that can be stored in a container.
    /// </summary>
    public interface IDataRow
    {
        /// <summary>
        /// Sets the container reference for this row, enabling navigation properties.
        /// </summary>
        void SetContainer(IDataContainer container);
    }

    /// <summary>
    /// Base interface for data containers.
    /// </summary>
    public interface IDataContainer
    {
        /// <summary>
        /// Gets or sets the root directory for resolving relative file paths.
        /// </summary>
        string RootDirectory { get; set; }
    }

    /// <summary>
    /// Interface for a data table that holds a collection of rows.
    /// </summary>
    /// <typeparam name="TRow">The type of rows in this table.</typeparam>
    public interface IDataTable<TRow> : IEnumerable<TRow> where TRow : class, IDataRow
    {
        /// <summary>
        /// Gets all rows in the table.
        /// </summary>
        IReadOnlyList<TRow> All { get; }

        /// <summary>
        /// Gets the number of rows in the table.
        /// </summary>
        int Count { get; }
    }

    /// <summary>
    /// A unique index that maps a key to a single row.
    /// Used for primary_key and unique constraints.
    /// </summary>
    /// <typeparam name="TKey">The type of the key.</typeparam>
    /// <typeparam name="TRow">The type of the row.</typeparam>
    public class UniqueIndex<TKey, TRow> where TKey : notnull where TRow : class
    {
        private readonly Dictionary<TKey, TRow> _index;

        public UniqueIndex()
        {
            _index = new Dictionary<TKey, TRow>();
        }

        public UniqueIndex(IEqualityComparer<TKey> comparer)
        {
            _index = new Dictionary<TKey, TRow>(comparer);
        }

        /// <summary>
        /// Gets a row by its key. Returns null if not found.
        /// </summary>
        public TRow? this[TKey key]
        {
            get
            {
                _index.TryGetValue(key, out var row);
                return row;
            }
        }

        /// <summary>
        /// Tries to get a row by its key.
        /// </summary>
        public bool TryGetValue(TKey key, out TRow? row)
        {
            return _index.TryGetValue(key, out row);
        }

        /// <summary>
        /// Checks if the index contains the specified key.
        /// </summary>
        public bool ContainsKey(TKey key)
        {
            return _index.ContainsKey(key);
        }

        /// <summary>
        /// Gets the number of entries in the index.
        /// </summary>
        public int Count => _index.Count;

        /// <summary>
        /// Gets all keys in the index.
        /// </summary>
        public IEnumerable<TKey> Keys => _index.Keys;

        /// <summary>
        /// Gets all values in the index.
        /// </summary>
        public IEnumerable<TRow> Values => _index.Values;

        /// <summary>
        /// Adds a row to the index. Internal use only.
        /// </summary>
        internal void Add(TKey key, TRow row)
        {
            _index[key] = row;
        }

        /// <summary>
        /// Clears all entries from the index. Internal use only.
        /// </summary>
        internal void Clear()
        {
            _index.Clear();
        }
    }

    /// <summary>
    /// A group index that maps a key to multiple rows.
    /// Used for foreign_key relationships and index constraints.
    /// </summary>
    /// <typeparam name="TKey">The type of the key.</typeparam>
    /// <typeparam name="TRow">The type of the row.</typeparam>
    public class GroupIndex<TKey, TRow> where TKey : notnull where TRow : class
    {
        private readonly Dictionary<TKey, List<TRow>> _index;
        private static readonly IReadOnlyList<TRow> EmptyList = new List<TRow>().AsReadOnly();

        public GroupIndex()
        {
            _index = new Dictionary<TKey, List<TRow>>();
        }

        public GroupIndex(IEqualityComparer<TKey> comparer)
        {
            _index = new Dictionary<TKey, List<TRow>>(comparer);
        }

        /// <summary>
        /// Gets all rows with the specified key. Returns empty list if not found.
        /// </summary>
        public IReadOnlyList<TRow> this[TKey key]
        {
            get
            {
                if (_index.TryGetValue(key, out var list))
                {
                    return list.AsReadOnly();
                }
                return EmptyList;
            }
        }

        /// <summary>
        /// Tries to get rows by key.
        /// </summary>
        public bool TryGetValue(TKey key, out IReadOnlyList<TRow> rows)
        {
            if (_index.TryGetValue(key, out var list))
            {
                rows = list.AsReadOnly();
                return true;
            }
            rows = EmptyList;
            return false;
        }

        /// <summary>
        /// Checks if the index contains the specified key.
        /// </summary>
        public bool ContainsKey(TKey key)
        {
            return _index.ContainsKey(key);
        }

        /// <summary>
        /// Gets the number of distinct keys in the index.
        /// </summary>
        public int Count => _index.Count;

        /// <summary>
        /// Gets all keys in the index.
        /// </summary>
        public IEnumerable<TKey> Keys => _index.Keys;

        /// <summary>
        /// Adds a row to the index. Internal use only.
        /// </summary>
        internal void Add(TKey key, TRow row)
        {
            if (!_index.TryGetValue(key, out var list))
            {
                list = new List<TRow>();
                _index[key] = list;
            }
            list.Add(row);
        }

        /// <summary>
        /// Clears all entries from the index. Internal use only.
        /// </summary>
        internal void Clear()
        {
            _index.Clear();
        }
    }

    /// <summary>
    /// Base class for data tables with common functionality.
    /// </summary>
    /// <typeparam name="TRow">The type of rows in this table.</typeparam>
    public abstract class DataTableBase<TRow> : IDataTable<TRow> where TRow : class, IDataRow
    {
        protected readonly List<TRow> _rows = new();
        protected IDataContainer? _container;

        /// <summary>
        /// Gets all rows in the table.
        /// </summary>
        public IReadOnlyList<TRow> All => _rows.AsReadOnly();

        /// <summary>
        /// Gets the number of rows in the table.
        /// </summary>
        public int Count => _rows.Count;

        /// <summary>
        /// Sets the container reference for this table.
        /// </summary>
        public void SetContainer(IDataContainer container)
        {
            _container = container;
        }

        /// <summary>
        /// Adds a row to the table and updates all indexes.
        /// </summary>
        protected void AddRowInternal(TRow row)
        {
            if (_container != null)
            {
                row.SetContainer(_container);
            }
            _rows.Add(row);
            OnRowAdded(row);
        }

        /// <summary>
        /// Called when a row is added. Override to update indexes.
        /// </summary>
        protected abstract void OnRowAdded(TRow row);

        /// <summary>
        /// Clears all rows and indexes.
        /// </summary>
        public virtual void Clear()
        {
            _rows.Clear();
            OnCleared();
        }

        /// <summary>
        /// Called when the table is cleared. Override to clear indexes.
        /// </summary>
        protected abstract void OnCleared();

        public IEnumerator<TRow> GetEnumerator() => _rows.GetEnumerator();
        IEnumerator IEnumerable.GetEnumerator() => GetEnumerator();
    }
}
