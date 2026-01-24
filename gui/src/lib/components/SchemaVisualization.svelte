<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import dagre from "dagre";

  interface FieldVisualization {
    name: string;
    type: string;
    isOptional: boolean;
    isList: boolean;
    isPrimaryKey: boolean;
    isUnique: boolean;
    isForeignKey: boolean;
    constraints: string[];
  }

  interface ReferenceInfo {
    tableName: string;
    tableFqn: string;
    fieldName: string;
    alias: string | null;
  }

  interface IndexVisualization {
    name: string;
    fields: string[];
    isUnique: boolean;
  }

  interface TableMetadata {
    datasource: string | null;
    cacheStrategy: string | null;
    isReadonly: boolean;
    softDeleteField: string | null;
  }

  interface TableVisualization {
    name: string;
    fqn: string;
    namespace: string;
    fields: FieldVisualization[];
    referencedBy: ReferenceInfo[];
    references: ReferenceInfo[];
    indexes: IndexVisualization[];
    metadata: TableMetadata;
  }

  interface SchemaStats {
    tableCount: number;
    fieldCount: number;
    relationCount: number;
    namespaceCount: number;
    tablesByNamespace: Record<string, number>;
  }

  interface SchemaVisualization {
    tables: TableVisualization[];
    stats: SchemaStats;
  }

  interface TablePosition {
    x: number;
    y: number;
    width: number;
    height: number;
  }

  let { schemaPath }: { schemaPath: string } = $props();

  let visualization = $state<SchemaVisualization | null>(null);
  let selectedTable = $state<TableVisualization | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(false);
  let searchQuery = $state("");
  let viewMode = $state<"detail" | "diagram">("detail");

  // Diagram state
  let diagramScale = $state(1);
  let diagramOffsetX = $state(0);
  let diagramOffsetY = $state(0);
  let isDragging = $state(false);
  let dragStartX = $state(0);
  let dragStartY = $state(0);

  // Filtered tables based on search
  let filteredTables = $derived(
    visualization?.tables.filter(
      (t) =>
        t.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
        t.fqn.toLowerCase().includes(searchQuery.toLowerCase()) ||
        t.namespace.toLowerCase().includes(searchQuery.toLowerCase())
    ) ?? []
  );

  // Group tables by namespace
  let tablesByNamespace = $derived(() => {
    const groups: Record<string, TableVisualization[]> = {};
    for (const table of filteredTables) {
      const ns = table.namespace || "(root)";
      if (!groups[ns]) groups[ns] = [];
      groups[ns].push(table);
    }
    return groups;
  });

  // Layout direction state
  let layoutDirection = $state<"LR" | "TB">("LR"); // Left-to-Right or Top-to-Bottom

  // Calculate table positions for diagram using the shared Dagre graph
  let tablePositions = $derived(() => {
    const g = dagreGraph();
    if (!g || !visualization) return new Map<string, TablePosition>();

    const positions = new Map<string, TablePosition>();

    for (const table of visualization.tables) {
      const node = g.node(table.fqn);
      if (node) {
        const height = getBoxHeight(table);
        positions.set(table.fqn, {
          x: node.x - boxWidth / 2,
          y: node.y - height / 2,
          width: boxWidth,
          height: height,
        });
      }
    }

    return positions;
  });

  // Diagram settings
  let showFields = $state(true);
  const boxWidth = 180;
  const headerHeight = 28;
  const fieldHeight = 18;
  const minBoxHeight = 60;
  const maxFieldsToShow = 8; // Limit fields shown to avoid huge boxes

  // Calculate box height based on fields
  function getBoxHeight(table: TableVisualization): number {
    if (!showFields) return minBoxHeight;
    const fieldCount = Math.min(table.fields.length, maxFieldsToShow);
    return headerHeight + fieldCount * fieldHeight + 10;
  }

  // Dagre graph instance for edge routing
  let dagreGraph = $derived(() => {
    if (!visualization) return null;

    const tables = visualization.tables;

    const g = new dagre.graphlib.Graph();
    g.setGraph({
      rankdir: layoutDirection,
      nodesep: 60,
      ranksep: 80,
      marginx: 40,
      marginy: 40,
    });
    g.setDefaultEdgeLabel(() => ({}));

    for (const table of tables) {
      const height = getBoxHeight(table);
      g.setNode(table.fqn, { width: boxWidth, height });
    }

    for (const table of tables) {
      for (const ref of table.references) {
        g.setEdge(table.fqn, ref.tableFqn);
      }
    }

    dagre.layout(g);
    return g;
  });

  // Calculate arrows for diagram using Dagre edge points
  let arrows = $derived(() => {
    const g = dagreGraph();
    if (!g || !visualization) return [];

    const result: {
      fromFqn: string;
      toFqn: string;
      points: { x: number; y: number }[];
      fieldName: string;
    }[] = [];

    for (const table of visualization.tables) {
      for (const ref of table.references) {
        const edge = g.edge(table.fqn, ref.tableFqn);
        if (edge && edge.points) {
          result.push({
            fromFqn: table.fqn,
            toFqn: ref.tableFqn,
            points: edge.points,
            fieldName: ref.fieldName,
          });
        }
      }
    }

    return result;
  });

  // Calculate SVG viewBox
  let svgViewBox = $derived(() => {
    const positions = tablePositions();
    if (positions.size === 0) return "0 0 800 600";

    let maxX = 0;
    let maxY = 0;

    for (const pos of positions.values()) {
      maxX = Math.max(maxX, pos.x + pos.width + 40);
      maxY = Math.max(maxY, pos.y + pos.height + 40);
    }

    return `0 0 ${maxX} ${maxY}`;
  });

  async function loadVisualization() {
    if (!schemaPath) return;

    loading = true;
    error = null;

    try {
      visualization = await invoke<SchemaVisualization>(
        "get_schema_visualization",
        { schemaPath }
      );
      if (visualization.tables.length > 0) {
        selectedTable = visualization.tables[0];
      }
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  function selectTable(table: TableVisualization) {
    selectedTable = table;
  }

  function navigateToTable(fqn: string) {
    const table = visualization?.tables.find((t) => t.fqn === fqn);
    if (table) {
      selectedTable = table;
    }
  }

  function getFieldTypeDisplay(field: FieldVisualization): string {
    let display = field.type;
    if (field.isList) display += "[]";
    if (field.isOptional) display += "?";
    return display;
  }

  function getFieldBadges(field: FieldVisualization): string[] {
    const badges: string[] = [];
    if (field.isPrimaryKey) badges.push("PK");
    if (field.isForeignKey) badges.push("FK");
    if (field.isUnique && !field.isPrimaryKey) badges.push("UK");
    return badges;
  }

  // Diagram pan/zoom handlers
  function handleWheel(e: WheelEvent) {
    e.preventDefault();
    const delta = e.deltaY > 0 ? 0.9 : 1.1;
    diagramScale = Math.max(0.25, Math.min(3, diagramScale * delta));
  }

  function handleMouseDown(e: MouseEvent) {
    if (e.button === 0) {
      isDragging = true;
      dragStartX = e.clientX - diagramOffsetX;
      dragStartY = e.clientY - diagramOffsetY;
    }
  }

  function handleMouseMove(e: MouseEvent) {
    if (isDragging) {
      diagramOffsetX = e.clientX - dragStartX;
      diagramOffsetY = e.clientY - dragStartY;
    }
  }

  function handleMouseUp() {
    isDragging = false;
  }

  function resetDiagramView() {
    diagramScale = 1;
    diagramOffsetX = 0;
    diagramOffsetY = 0;
  }

  // Calculate arrow path from Dagre edge points
  function getArrowPathFromPoints(points: { x: number; y: number }[]): string {
    if (points.length === 0) return "";
    if (points.length === 1) return `M ${points[0].x} ${points[0].y}`;

    // Start with first point
    let path = `M ${points[0].x} ${points[0].y}`;

    if (points.length === 2) {
      // Simple line
      path += ` L ${points[1].x} ${points[1].y}`;
    } else {
      // Use smooth curve through points
      for (let i = 1; i < points.length; i++) {
        const prev = points[i - 1];
        const curr = points[i];

        if (i === 1) {
          // First segment: quadratic curve
          const midX = (prev.x + curr.x) / 2;
          const midY = (prev.y + curr.y) / 2;
          path += ` Q ${prev.x} ${prev.y}, ${midX} ${midY}`;
        } else if (i === points.length - 1) {
          // Last segment: curve to end
          path += ` T ${curr.x} ${curr.y}`;
        } else {
          // Middle segments
          const midX = (prev.x + curr.x) / 2;
          const midY = (prev.y + curr.y) / 2;
          path += ` T ${midX} ${midY}`;
        }
      }
    }

    return path;
  }

  // Load on mount or when path changes
  $effect(() => {
    if (schemaPath) {
      loadVisualization();
    }
  });
</script>

<div class="visualization-container">
  {#if loading}
    <div class="loading">Loading schema visualization...</div>
  {:else if error}
    <div class="error">
      <p>Failed to load visualization:</p>
      <pre>{error}</pre>
      <button onclick={loadVisualization}>Retry</button>
    </div>
  {:else if visualization}
    <!-- Stats bar with view toggle -->
    <div class="stats-bar">
      <div class="stats-left">
        <span class="stat">
          <strong>{visualization.stats.tableCount}</strong> Tables
        </span>
        <span class="stat">
          <strong>{visualization.stats.fieldCount}</strong> Fields
        </span>
        <span class="stat">
          <strong>{visualization.stats.relationCount}</strong> Relations
        </span>
        <span class="stat">
          <strong>{visualization.stats.namespaceCount}</strong> Namespaces
        </span>
      </div>
      <div class="view-toggle">
        <button
          class="toggle-btn"
          class:active={viewMode === "detail"}
          onclick={() => (viewMode = "detail")}
        >
          Detail
        </button>
        <button
          class="toggle-btn"
          class:active={viewMode === "diagram"}
          onclick={() => (viewMode = "diagram")}
        >
          Diagram
        </button>
      </div>
    </div>

    {#if viewMode === "detail"}
      <div class="main-content">
        <!-- Table list sidebar -->
        <div class="table-list">
          <input
            type="text"
            class="search-input"
            placeholder="Search tables..."
            bind:value={searchQuery}
          />

          <div class="table-tree">
            {#each Object.entries(tablesByNamespace()) as [namespace, tables]}
              <div class="namespace-group">
                <div class="namespace-header">{namespace}</div>
                {#each tables as table}
                  <button
                    class="table-item"
                    class:active={selectedTable?.fqn === table.fqn}
                    onclick={() => selectTable(table)}
                  >
                    <span class="table-name">{table.name}</span>
                    {#if table.referencedBy.length > 0 || table.references.length > 0}
                      <span class="relation-count">
                        {table.referencedBy.length + table.references.length}
                      </span>
                    {/if}
                  </button>
                {/each}
              </div>
            {/each}
          </div>
        </div>

        <!-- Main visualization area with three columns -->
        {#if selectedTable}
          <div class="detail-view">
            <!-- Left: Referenced By (tables that reference this table) -->
            <div class="ref-column left">
              <h3>Referenced By</h3>
              {#if selectedTable.referencedBy.length === 0}
                <div class="empty-refs">No incoming references</div>
              {:else}
                {#each selectedTable.referencedBy as ref}
                  <button
                    class="ref-card"
                    onclick={() => navigateToTable(ref.tableFqn)}
                  >
                    <div class="ref-table-name">{ref.tableName}</div>
                    <div class="ref-field">
                      via <code>{ref.fieldName}</code>
                      {#if ref.alias}
                        <span class="ref-alias">as {ref.alias}</span>
                      {/if}
                    </div>
                    <div class="ref-arrow">→</div>
                  </button>
                {/each}
              {/if}
            </div>

            <!-- Center: Current table details -->
            <div class="table-detail">
              <div class="table-header">
                <h2>{selectedTable.name}</h2>
                <div class="table-fqn">{selectedTable.fqn}</div>
                {#if selectedTable.metadata.datasource}
                  <span class="metadata-badge datasource">
                    @{selectedTable.metadata.datasource}
                  </span>
                {/if}
                {#if selectedTable.metadata.isReadonly}
                  <span class="metadata-badge readonly">readonly</span>
                {/if}
                {#if selectedTable.metadata.cacheStrategy}
                  <span class="metadata-badge cache">
                    cache: {selectedTable.metadata.cacheStrategy}
                  </span>
                {/if}
                {#if selectedTable.metadata.softDeleteField}
                  <span class="metadata-badge soft-delete">
                    soft_delete: {selectedTable.metadata.softDeleteField}
                  </span>
                {/if}
              </div>

              <div class="fields-section">
                <h4>Fields</h4>
                <table class="fields-table">
                  <thead>
                    <tr>
                      <th>Name</th>
                      <th>Type</th>
                      <th>Constraints</th>
                    </tr>
                  </thead>
                  <tbody>
                    {#each selectedTable.fields as field}
                      <tr class:pk={field.isPrimaryKey} class:fk={field.isForeignKey}>
                        <td class="field-name">
                          {field.name}
                          {#each getFieldBadges(field) as badge}
                            <span class="field-badge {badge.toLowerCase()}">{badge}</span>
                          {/each}
                        </td>
                        <td class="field-type">
                          <code>{getFieldTypeDisplay(field)}</code>
                        </td>
                        <td class="field-constraints">
                          {#each field.constraints as constraint}
                            <span class="constraint">{constraint}</span>
                          {/each}
                        </td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </div>

              {#if selectedTable.indexes.length > 0}
                <div class="indexes-section">
                  <h4>Indexes</h4>
                  <div class="indexes-list">
                    {#each selectedTable.indexes as idx}
                      <div class="index-item">
                        <span class="index-name">{idx.name}</span>
                        {#if idx.isUnique}
                          <span class="index-badge">UNIQUE</span>
                        {/if}
                        <span class="index-fields">
                          ({idx.fields.join(", ")})
                        </span>
                      </div>
                    {/each}
                  </div>
                </div>
              {/if}
            </div>

            <!-- Right: References (tables this table references) -->
            <div class="ref-column right">
              <h3>References</h3>
              {#if selectedTable.references.length === 0}
                <div class="empty-refs">No outgoing references</div>
              {:else}
                {#each selectedTable.references as ref}
                  <button
                    class="ref-card"
                    onclick={() => navigateToTable(ref.tableFqn)}
                  >
                    <div class="ref-arrow">→</div>
                    <div class="ref-table-name">{ref.tableName}</div>
                    <div class="ref-field">
                      via <code>{ref.fieldName}</code>
                      {#if ref.alias}
                        <span class="ref-alias">as {ref.alias}</span>
                      {/if}
                    </div>
                  </button>
                {/each}
              {/if}
            </div>
          </div>
        {:else}
          <div class="no-selection">
            <p>Select a table to view its details and references</p>
          </div>
        {/if}
      </div>
    {:else}
      <!-- Diagram View -->
      <div class="diagram-container">
        <div class="diagram-controls">
          <button onclick={() => (diagramScale = Math.min(3, diagramScale * 1.2))}>+</button>
          <span class="zoom-level">{Math.round(diagramScale * 100)}%</span>
          <button onclick={() => (diagramScale = Math.max(0.25, diagramScale * 0.8))}>−</button>
          <button onclick={resetDiagramView}>Reset</button>
          <span class="control-separator"></span>
          <button
            class="layout-btn"
            class:active={layoutDirection === "LR"}
            onclick={() => (layoutDirection = "LR")}
            title="Left to Right"
          >→</button>
          <button
            class="layout-btn"
            class:active={layoutDirection === "TB"}
            onclick={() => (layoutDirection = "TB")}
            title="Top to Bottom"
          >↓</button>
          <span class="control-separator"></span>
          <label class="checkbox-label">
            <input type="checkbox" bind:checked={showFields} />
            Fields
          </label>
        </div>
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="diagram-viewport"
          onwheel={handleWheel}
          onmousedown={handleMouseDown}
          onmousemove={handleMouseMove}
          onmouseup={handleMouseUp}
          onmouseleave={handleMouseUp}
        >
          <svg
            class="diagram-svg"
            viewBox={svgViewBox()}
            style="transform: scale({diagramScale}) translate({diagramOffsetX / diagramScale}px, {diagramOffsetY / diagramScale}px)"
          >
            <!-- Arrow marker definition -->
            <defs>
              <marker
                id="arrowhead"
                markerWidth="10"
                markerHeight="7"
                refX="10"
                refY="3.5"
                orient="auto"
              >
                <polygon points="0 0, 10 3.5, 0 7" fill="var(--accent, #3b82f6)" />
              </marker>
            </defs>

            <!-- Draw arrows first (behind boxes) -->
            {#each arrows() as arrow}
              <path
                class="relation-arrow"
                d={getArrowPathFromPoints(arrow.points)}
                marker-end="url(#arrowhead)"
              />
            {/each}

            <!-- Draw table boxes -->
            {#each visualization.tables as table}
              {@const pos = tablePositions().get(table.fqn)}
              {#if pos}
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
                <g
                  class="table-box"
                  class:selected={selectedTable?.fqn === table.fqn}
                  onclick={() => selectTable(table)}
                >
                  <!-- Box background -->
                  <rect
                    x={pos.x}
                    y={pos.y}
                    width={pos.width}
                    height={pos.height}
                    rx="6"
                    ry="6"
                    class="table-rect"
                  />

                  <!-- Table name header -->
                  <rect
                    x={pos.x}
                    y={pos.y}
                    width={pos.width}
                    height="28"
                    rx="6"
                    ry="6"
                    class="table-header-rect"
                  />
                  <rect
                    x={pos.x}
                    y={pos.y + 22}
                    width={pos.width}
                    height="6"
                    class="table-header-rect"
                  />

                  <text
                    x={pos.x + pos.width / 2}
                    y={pos.y + 18}
                    class="table-name-text"
                  >
                    {table.name}
                  </text>

                  <!-- Fields list -->
                  {#if showFields}
                    {#each table.fields.slice(0, maxFieldsToShow) as field, i}
                      <g class="field-row">
                        <!-- Field badges (PK, FK) -->
                        {#if field.isPrimaryKey}
                          <rect
                            x={pos.x + 4}
                            y={pos.y + headerHeight + 2 + i * fieldHeight}
                            width="16"
                            height="12"
                            rx="2"
                            class="field-badge-rect pk"
                          />
                          <text
                            x={pos.x + 12}
                            y={pos.y + headerHeight + 11 + i * fieldHeight}
                            class="field-badge-text"
                          >PK</text>
                        {:else if field.isForeignKey}
                          <rect
                            x={pos.x + 4}
                            y={pos.y + headerHeight + 2 + i * fieldHeight}
                            width="16"
                            height="12"
                            rx="2"
                            class="field-badge-rect fk"
                          />
                          <text
                            x={pos.x + 12}
                            y={pos.y + headerHeight + 11 + i * fieldHeight}
                            class="field-badge-text"
                          >FK</text>
                        {/if}
                        <!-- Field name -->
                        <text
                          x={pos.x + (field.isPrimaryKey || field.isForeignKey ? 24 : 6)}
                          y={pos.y + headerHeight + 12 + i * fieldHeight}
                          class="diagram-field-name"
                        >
                          {field.name}
                        </text>
                        <!-- Field type -->
                        <text
                          x={pos.x + pos.width - 6}
                          y={pos.y + headerHeight + 12 + i * fieldHeight}
                          class="diagram-field-type"
                        >
                          {field.type}{field.isList ? "[]" : ""}{field.isOptional ? "?" : ""}
                        </text>
                      </g>
                    {/each}
                    {#if table.fields.length > maxFieldsToShow}
                      <text
                        x={pos.x + pos.width / 2}
                        y={pos.y + headerHeight + maxFieldsToShow * fieldHeight + 8}
                        class="more-fields-text"
                      >
                        +{table.fields.length - maxFieldsToShow} more...
                      </text>
                    {/if}
                  {:else}
                    <!-- Collapsed view: just show count and namespace -->
                    <text
                      x={pos.x + pos.width / 2}
                      y={pos.y + 42}
                      class="field-count-text"
                    >
                      {table.fields.length} fields
                    </text>
                    {#if table.namespace}
                      <text
                        x={pos.x + pos.width / 2}
                        y={pos.y + 55}
                        class="namespace-text"
                      >
                        {table.namespace}
                      </text>
                    {/if}
                  {/if}

                  <!-- Reference indicators -->
                  {#if table.referencedBy.length > 0}
                    <circle
                      cx={pos.x}
                      cy={pos.y + pos.height / 2}
                      r="10"
                      class="ref-indicator incoming"
                    />
                    <text
                      x={pos.x}
                      y={pos.y + pos.height / 2 + 4}
                      class="ref-count-text"
                    >
                      {table.referencedBy.length}
                    </text>
                  {/if}

                  {#if table.references.length > 0}
                    <circle
                      cx={pos.x + pos.width}
                      cy={pos.y + pos.height / 2}
                      r="10"
                      class="ref-indicator outgoing"
                    />
                    <text
                      x={pos.x + pos.width}
                      y={pos.y + pos.height / 2 + 4}
                      class="ref-count-text"
                    >
                      {table.references.length}
                    </text>
                  {/if}
                </g>
              {/if}
            {/each}
          </svg>
        </div>
      </div>
    {/if}
  {:else}
    <div class="empty-state">
      <p>Open a schema file to visualize its structure</p>
    </div>
  {/if}
</div>

<style>
  .visualization-container {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-primary);
  }

  .loading,
  .error,
  .empty-state,
  .no-selection {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-secondary);
    gap: 1rem;
  }

  .error pre {
    background: var(--bg-secondary);
    padding: 1rem;
    border-radius: 4px;
    max-width: 80%;
    overflow-x: auto;
  }

  .stats-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem 1rem;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
  }

  .stats-left {
    display: flex;
    gap: 1.5rem;
  }

  .stat {
    font-size: 0.875rem;
    color: var(--text-secondary);
  }

  .stat strong {
    color: var(--accent);
  }

  .view-toggle {
    display: flex;
    gap: 0.25rem;
    background: var(--bg-primary);
    padding: 0.25rem;
    border-radius: 6px;
  }

  .toggle-btn {
    padding: 0.375rem 0.75rem;
    font-size: 0.875rem;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    border-radius: 4px;
    cursor: pointer;
  }

  .toggle-btn:hover {
    background: var(--bg-hover);
  }

  .toggle-btn.active {
    background: var(--accent);
    color: white;
  }

  .main-content {
    display: flex;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .table-list {
    width: 220px;
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    background: var(--bg-secondary);
  }

  .search-input {
    margin: 0.5rem;
    padding: 0.5rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg-primary);
    color: var(--text-primary);
  }

  .table-tree {
    flex: 1;
    overflow-y: auto;
    padding: 0 0.5rem;
  }

  .namespace-group {
    margin-bottom: 0.5rem;
  }

  .namespace-header {
    font-size: 0.75rem;
    color: var(--text-secondary);
    padding: 0.5rem 0.5rem 0.25rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .table-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 0.375rem 0.5rem;
    background: transparent;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    color: var(--text-primary);
    font-size: 0.875rem;
    text-align: left;
  }

  .table-item:hover {
    background: var(--bg-hover);
  }

  .table-item.active {
    background: var(--accent);
    color: white;
  }

  .relation-count {
    font-size: 0.75rem;
    background: var(--bg-primary);
    padding: 0.125rem 0.375rem;
    border-radius: 10px;
    color: var(--text-secondary);
  }

  .table-item.active .relation-count {
    background: rgba(255, 255, 255, 0.2);
    color: white;
  }

  .detail-view {
    display: grid;
    grid-template-columns: 200px 1fr 200px;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .ref-column {
    display: flex;
    flex-direction: column;
    padding: 1rem;
    background: var(--bg-secondary);
    overflow-y: auto;
  }

  .ref-column.left {
    border-right: 1px solid var(--border);
  }

  .ref-column.right {
    border-left: 1px solid var(--border);
  }

  .ref-column h3 {
    font-size: 0.875rem;
    color: var(--text-secondary);
    margin: 0 0 0.75rem 0;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .empty-refs {
    color: var(--text-secondary);
    font-size: 0.875rem;
    font-style: italic;
    padding: 0.5rem;
  }

  .ref-card {
    display: flex;
    flex-direction: column;
    padding: 0.75rem;
    margin-bottom: 0.5rem;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 6px;
    cursor: pointer;
    text-align: left;
    color: var(--text-primary);
  }

  .ref-card:hover {
    border-color: var(--accent);
    background: var(--bg-hover);
  }

  .ref-table-name {
    font-weight: 600;
    font-size: 0.875rem;
  }

  .ref-field {
    font-size: 0.75rem;
    color: var(--text-secondary);
    margin-top: 0.25rem;
  }

  .ref-field code {
    background: var(--bg-secondary);
    padding: 0.125rem 0.25rem;
    border-radius: 2px;
  }

  .ref-alias {
    color: var(--accent);
  }

  .ref-arrow {
    font-size: 1rem;
    color: var(--accent);
    opacity: 0.5;
  }

  .left .ref-arrow {
    text-align: right;
    margin-top: 0.25rem;
  }

  .right .ref-arrow {
    margin-bottom: 0.25rem;
  }

  .table-detail {
    flex: 1;
    overflow-y: auto;
    padding: 1rem;
  }

  .table-header {
    margin-bottom: 1rem;
    padding-bottom: 1rem;
    border-bottom: 1px solid var(--border);
  }

  .table-header h2 {
    margin: 0;
    font-size: 1.25rem;
    color: var(--text-primary);
  }

  .table-fqn {
    font-size: 0.875rem;
    color: var(--text-secondary);
    margin-top: 0.25rem;
    font-family: monospace;
  }

  .metadata-badge {
    display: inline-block;
    font-size: 0.75rem;
    padding: 0.125rem 0.5rem;
    border-radius: 4px;
    margin-top: 0.5rem;
    margin-right: 0.25rem;
  }

  .metadata-badge.datasource {
    background: #3b82f620;
    color: #3b82f6;
  }

  .metadata-badge.readonly {
    background: #f59e0b20;
    color: #f59e0b;
  }

  .metadata-badge.cache {
    background: #10b98120;
    color: #10b981;
  }

  .metadata-badge.soft-delete {
    background: #ef444420;
    color: #ef4444;
  }

  .fields-section h4,
  .indexes-section h4 {
    font-size: 0.875rem;
    color: var(--text-secondary);
    margin: 0 0 0.5rem 0;
  }

  .fields-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.875rem;
  }

  .fields-table th {
    text-align: left;
    padding: 0.5rem;
    border-bottom: 1px solid var(--border);
    color: var(--text-secondary);
    font-weight: 500;
  }

  .fields-table td {
    padding: 0.5rem;
    border-bottom: 1px solid var(--border);
  }

  .fields-table tr.pk {
    background: var(--accent-light, rgba(59, 130, 246, 0.05));
  }

  .fields-table tr.fk {
    background: rgba(245, 158, 11, 0.05);
  }

  .field-name {
    font-weight: 500;
  }

  .field-badge {
    font-size: 0.625rem;
    padding: 0.125rem 0.25rem;
    border-radius: 2px;
    margin-left: 0.25rem;
    font-weight: 600;
  }

  .field-badge.pk {
    background: #3b82f6;
    color: white;
  }

  .field-badge.fk {
    background: #f59e0b;
    color: white;
  }

  .field-badge.uk {
    background: #10b981;
    color: white;
  }

  .field-type code {
    background: var(--bg-secondary);
    padding: 0.125rem 0.375rem;
    border-radius: 2px;
    font-size: 0.8125rem;
  }

  .field-constraints {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
  }

  .constraint {
    font-size: 0.75rem;
    background: var(--bg-secondary);
    padding: 0.125rem 0.375rem;
    border-radius: 2px;
    color: var(--text-secondary);
  }

  .indexes-section {
    margin-top: 1rem;
  }

  .indexes-list {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .index-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.875rem;
  }

  .index-name {
    font-weight: 500;
  }

  .index-badge {
    font-size: 0.625rem;
    background: #10b981;
    color: white;
    padding: 0.125rem 0.25rem;
    border-radius: 2px;
  }

  .index-fields {
    color: var(--text-secondary);
  }

  /* Diagram View Styles */
  .diagram-container {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
  }

  .diagram-controls {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 1rem;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
  }

  .diagram-controls button {
    padding: 0.25rem 0.5rem;
    font-size: 0.875rem;
    border: 1px solid var(--border);
    background: var(--bg-primary);
    color: var(--text-primary);
    border-radius: 4px;
    cursor: pointer;
  }

  .diagram-controls button:hover {
    background: var(--bg-hover);
  }

  .zoom-level {
    font-size: 0.875rem;
    color: var(--text-secondary);
    min-width: 50px;
    text-align: center;
  }

  .control-separator {
    width: 1px;
    height: 1.25rem;
    background: var(--border);
    margin: 0 0.25rem;
  }

  .layout-btn {
    font-size: 1rem;
    min-width: 2rem;
  }

  .layout-btn.active {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
  }

  .diagram-viewport {
    flex: 1;
    overflow: hidden;
    cursor: grab;
    background: var(--bg-primary);
    background-image: radial-gradient(circle, var(--border) 1px, transparent 1px);
    background-size: 20px 20px;
  }

  .diagram-viewport:active {
    cursor: grabbing;
  }

  .diagram-svg {
    width: 100%;
    height: 100%;
    transform-origin: 0 0;
  }

  .relation-arrow {
    fill: none;
    stroke: var(--accent, #3b82f6);
    stroke-width: 2;
    opacity: 0.6;
  }

  .table-box {
    cursor: pointer;
  }

  .table-box:hover .table-rect {
    stroke: var(--accent, #3b82f6);
    stroke-width: 2;
  }

  .table-box.selected .table-rect {
    stroke: var(--accent, #3b82f6);
    stroke-width: 3;
  }

  .table-rect {
    fill: var(--bg-secondary, #1e1e1e);
    stroke: var(--border, #333);
    stroke-width: 1;
  }

  .table-header-rect {
    fill: var(--accent, #3b82f6);
  }

  .table-name-text {
    fill: white;
    font-size: 12px;
    font-weight: 600;
    text-anchor: middle;
  }

  .field-count-text {
    fill: var(--text-secondary, #888);
    font-size: 11px;
    text-anchor: middle;
  }

  .namespace-text {
    fill: var(--text-secondary, #666);
    font-size: 10px;
    text-anchor: middle;
  }

  .datasource-text {
    fill: var(--accent, #3b82f6);
    font-size: 10px;
    text-anchor: middle;
  }

  .ref-indicator {
    stroke: white;
    stroke-width: 2;
  }

  .ref-indicator.incoming {
    fill: #f59e0b;
  }

  .ref-indicator.outgoing {
    fill: #3b82f6;
  }

  .ref-count-text {
    fill: white;
    font-size: 10px;
    font-weight: 600;
    text-anchor: middle;
  }

  /* Diagram field display styles */
  .diagram-field-name {
    fill: var(--text-primary, #ccc);
    font-size: 10px;
    font-family: monospace;
  }

  .diagram-field-type {
    fill: var(--text-secondary, #888);
    font-size: 9px;
    font-family: monospace;
    text-anchor: end;
  }

  .field-badge-rect.pk {
    fill: #3b82f6;
  }

  .field-badge-rect.fk {
    fill: #f59e0b;
  }

  .field-badge-text {
    fill: white;
    font-size: 7px;
    font-weight: 600;
    text-anchor: middle;
  }

  .more-fields-text {
    fill: var(--text-secondary, #666);
    font-size: 9px;
    text-anchor: middle;
    font-style: italic;
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.875rem;
    color: var(--text-secondary);
    cursor: pointer;
  }

  .checkbox-label input {
    cursor: pointer;
  }
</style>
