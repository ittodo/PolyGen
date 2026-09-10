<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { onDestroy, onMount, tick } from "svelte";
  import { createUniver, LocaleType, mergeLocales } from "@univerjs/presets";
  import { UniverSheetsCorePreset } from "@univerjs/preset-sheets-core";
  import { UniverSheetsDataValidationPreset } from "@univerjs/preset-sheets-data-validation";
  import "@univerjs/sheets-data-validation/facade";
  import "@univerjs/sheets-ui/facade";
  import koKR from "@univerjs/preset-sheets-core/locales/ko-KR";
  import type { ICellData, IRange, IWorksheetData } from "@univerjs/core";
  import { SetRangeValuesCommand } from "@univerjs/sheets";
  import type { FWorkbook } from "@univerjs/sheets/facade";
  import type { FUniver } from "@univerjs/presets";
  import JsonEditor from "./lib/JsonEditor.svelte";
  import {
    collectChangedCalculationUpdates,
    isBlockedUnmanagedCommand,
    POLYSHEET_UNIVER_UI_POLICY,
  } from "./lib/gridSync";
  import {
    classifyCell,
    classifySelection,
    type InspectorSelection,
  } from "./lib/selection";

  type FieldSummary = {
    name: string;
    typeName: string;
    isPrimaryKey: boolean;
    isOptional: boolean;
    isList: boolean;
    isEnum: boolean;
    isStruct: boolean;
    enumValues: string[];
    foreignKeyTarget?: string;
    foreignKeyField?: string;
    defaultValue?: string;
    maxLength?: number;
    rangeMin?: string;
    rangeMax?: string;
    regexPattern?: string;
  };
  type TableSummary = {
    fqn: string;
    name: string;
    isReadonly: boolean;
    jsonSource?: string;
    fields: FieldSummary[];
  };
  type SheetSummary = {
    id: string;
    name: string;
    kind: "data" | "calculation";
    definition?: string;
    rowCount: number;
    columnCount: number;
  };
  type Diagnostic = {
    severity: "error" | "warning";
    path: string;
    message: string;
  };
  type ProjectSummary = {
    name: string;
    root: string;
    schemaPath: string;
    isSaved: boolean;
    tables: TableSummary[];
    sheets: SheetSummary[];
    diagnostics: Diagnostic[];
    normalizationRequired: string[];
  };
  type RowRecord = { id: string; value: Record<string, unknown> };
  type RowChunk = { offset: number; total: number; rows: RowRecord[] };
  type CellEditPayload = {
    sheetId: string;
    rowId: string;
    fieldPath: string;
    value: unknown;
    formula?: string;
  };
  type DiffEntry = {
    kind: string;
    sheetId: string;
    sheetName: string;
    rowId?: string;
    path: string;
    before?: unknown;
    after?: unknown;
  };
  type DiffReport = { changes: DiffEntry[] };
  type FormulaDocument = {
    version: number;
    needs_recalculation: boolean;
    rows: Record<string, Record<string, string>>;
    cells: Record<string, string>;
  };
  type CalculationCell = { value?: unknown; formula?: string };
  type CalculationDocument = {
    version: number;
    needs_recalculation: boolean;
    row_order: string[];
    column_order: string[];
    cells: Record<string, CalculationCell>;
    formula_bindings?: Record<string, unknown>;
  };
  type NormalizationPreview = { path: string; before: string; after: string };
  type MergeConflict = {
    sheet_id: string;
    row_id?: string;
    path: string;
    base?: unknown;
    ours?: unknown;
    theirs?: unknown;
    reason: string;
  };
  type MergeReport = {
    conflicts: MergeConflict[];
    recalculation_required: boolean;
  };
  type InspectorField = FieldSummary & {
    path: string;
    required: boolean;
    inputExample: unknown;
  };
  type RowDraft = {
    json: string;
    missingRequired: string[];
    fields: InspectorField[];
    editable: boolean;
    readonlyReason?: string;
  };
  type RowComparisonValue = {
    rowId: string;
    present: boolean;
    value: unknown;
    formula?: string;
  };
  type RowComparisonField = {
    path: string;
    typeName: string;
    isList: boolean;
    allEqual: boolean;
    values: RowComparisonValue[];
  };
  type RowComparison = {
    rowIds: string[];
    fields: RowComparisonField[];
  };
  type InsertedRow = { rowId: string; project: ProjectSummary };
  type FormulaHint = {
    path: Array<string | number>;
    formula: string;
    value: unknown;
  };
  type RecentProject = {
    name: string;
    path: string;
    lastOpenedAt: string;
  };

  let projectPath = "";
  let recentProjects: RecentProject[] = [];
  let project: ProjectSummary | null = null;
  let selectedSheetId = "";
  let rightTab: "json" | "issues" | "diff" | "merge" | "normalization" = "json";
  let jsonText = "";
  let status = "PolySheet 프로젝트를 여세요.";
  let error = "";
  let busy = false;
  let gridHost: HTMLDivElement;
  let univer: { dispose(): void } | null = null;
  let univerAPI: FUniver | null = null;
  let workbook: FWorkbook | null = null;
  let selectionSubscriptions: Array<{ dispose(): void }> = [];
  let gridSelectionCleanup: (() => void) | null = null;
  let gridSyncTimers = new Map<string, ReturnType<typeof setTimeout>>();
  let rowsBySheet = new Map<string, RowRecord[]>();
  let formulasBySheet = new Map<string, FormulaDocument>();
  let calculationsBySheet = new Map<string, CalculationDocument>();
  let diffReport: DiffReport = { changes: [] };
  let normalizationPreview: NormalizationPreview[] = [];
  let mergeReport: MergeReport | null = null;
  let gitRefs: string[] = [];
  let diffBase = "HEAD";
  let diffTarget = "WORKTREE";
  let mergeBase = "HEAD~1";
  let mergeOurs = "WORKTREE";
  let mergeTheirs = "HEAD";
  let bindDefinition = "";
  let newSheetName = "";
  let createMode = false;
  let projectName = "Game Data";
  let schemaPath = "";
  let sourcesPath = "";
  let dataRoot = "";
  let projectDialogOpen = false;
  let projectDialogMode: "save" | "schema" = "save";
  let saveAsPath = "";
  let inspectorSelection: InspectorSelection = { kind: "create" };
  let inspectorMode: "context" | "whole" = "context";
  let inspectorBaseline = "";
  let pendingInspectorSelection: InspectorSelection | null = null;
  let rowDraft: RowDraft | null = null;
  let rowComparison: RowComparison | null = null;
  let differencesOnly = true;
  let selectionTrace = "선택 이벤트 대기 중";
  let jsonEditor: JsonEditor | undefined;
  const NULL_VALUE = "⟨null⟩";
  const RECENT_PROJECTS_KEY = "polysheet.recent-projects.v1";
  const APP_VERSION = "0.1.17";

  $: selectedSheet = project?.sheets.find((sheet) => sheet.id === selectedSheetId);
  $: selectedTable = project?.tables.find(
    (table) => table.fqn === selectedSheet?.definition,
  );
  $: inspectorDirty =
    (inspectorSelection.kind === "row" || inspectorSelection.kind === "create") &&
    jsonText !== inspectorBaseline;
  $: selectedInspectorField =
    inspectorFieldFor(inspectorSelection);
  $: visibleComparisonFields =
    differencesOnly
      ? rowComparison?.fields.filter((field) => !field.allEqual) ?? []
      : rowComparison?.fields ?? [];
  $: inspectorFormulaHints = buildInspectorFormulaHints(
    inspectorMode,
    inspectorSelection,
    selectedSheetId,
    rowsBySheet,
    formulasBySheet,
  );

  async function run<T>(operation: () => Promise<T>): Promise<T | undefined> {
    busy = true;
    error = "";
    try {
      return await operation();
    } catch (cause) {
      error = String(cause);
      return undefined;
    } finally {
      busy = false;
    }
  }

  function inspectorFieldFor(selection: InspectorSelection) {
    if (selection.kind !== "field") return undefined;
    return rowDraft?.fields.find((field) => field.path === selection.fieldPath);
  }

  function buildInspectorFormulaHints(
    mode: "context" | "whole",
    selection: InspectorSelection,
    sheetId: string,
    rows: Map<string, RowRecord[]>,
    formulas: Map<string, FormulaDocument>,
  ): FormulaHint[] {
    const document = formulas.get(sheetId);
    if (!document) return [];
    if (mode === "context") {
      if (selection.kind !== "row") return [];
      return Object.entries(document.rows[selection.rowId] ?? {}).map(
        ([fieldPath, formula]) => ({
          path: fieldPath.split("."),
          formula,
          value: valueAtFieldPath(
            rows.get(sheetId)?.find((row) => row.id === selection.rowId)?.value,
            fieldPath,
          ),
        }),
      );
    }
    return (rows.get(sheetId) ?? []).flatMap((row, rowIndex) =>
      Object.entries(document.rows[row.id] ?? {}).map(
        ([fieldPath, formula]) => ({
          path: [rowIndex, ...fieldPath.split(".")],
          formula,
          value: valueAtFieldPath(row.value, fieldPath),
        }),
      ),
    );
  }

  function valueAtFieldPath(
    root: Record<string, unknown> | undefined,
    fieldPath: string,
  ) {
    let current: unknown = root;
    for (const segment of fieldPath.split(".")) {
      if (!current || typeof current !== "object" || Array.isArray(current)) {
        return undefined;
      }
      current = (current as Record<string, unknown>)[segment];
    }
    return current;
  }

  function inlineJsonValue(value: unknown) {
    const serialized = JSON.stringify(value);
    return serialized === undefined ? "undefined" : serialized;
  }

  async function openProject() {
    const summary = await run(() =>
      invoke<ProjectSummary>("open_project", { root: projectPath }),
    );
    if (!summary) return;
    project = summary;
    projectPath = summary.root;
    selectedSheetId = summary.sheets[0]?.id ?? "";
    status = `${summary.name} — ${summary.sheets.length}개 시트`;
    gitRefs = (await run(() => invoke<string[]>("list_git_refs"))) ?? [];
    normalizationPreview =
      (await run(() => invoke<NormalizationPreview[]>("get_normalization_preview"))) ?? [];
    rememberRecentProject(summary);
    await rebuildWorkbook();
    if (selectedSheetId) await selectSheet(selectedSheetId);
  }

  async function openRecentProject(path: string) {
    projectPath = path;
    createMode = false;
    await openProject();
  }

  function rememberRecentProject(summary: ProjectSummary) {
    if (!summary.isSaved || !summary.root) return;
    const normalized = normalizeRecentPath(summary.root);
    recentProjects = [
      {
        name: summary.name,
        path: summary.root,
        lastOpenedAt: new Date().toISOString(),
      },
      ...recentProjects.filter(
        (recent) => normalizeRecentPath(recent.path) !== normalized,
      ),
    ].slice(0, 8);
    persistRecentProjects();
  }

  function removeRecentProject(path: string) {
    const normalized = normalizeRecentPath(path);
    recentProjects = recentProjects.filter(
      (recent) => normalizeRecentPath(recent.path) !== normalized,
    );
    persistRecentProjects();
  }

  function normalizeRecentPath(path: string) {
    return path.replace(/[\\/]+$/, "").toLocaleLowerCase();
  }

  function persistRecentProjects() {
    localStorage.setItem(RECENT_PROJECTS_KEY, JSON.stringify(recentProjects));
  }

  async function createProject() {
    const summary = await run(() =>
      invoke<ProjectSummary>("create_unsaved_project", {
        name: projectName,
      }),
    );
    if (!summary) return;
    project = summary;
    selectedSheetId = summary.sheets[0]?.id ?? "";
    status = `${summary.name} — 아직 저장되지 않은 새 문서`;
    await rebuildWorkbook();
    if (selectedSheetId) await selectSheet(selectedSheetId);
  }

  async function pickDirectory(target: "project" | "save" | "data") {
    const selected = await openDialog({
      directory: true,
      multiple: false,
      title:
        target === "project"
          ? "PolySheet 프로젝트 폴더 선택"
          : target === "save"
            ? "저장할 PolySheet 프로젝트 폴더 선택"
            : "데이터 루트 폴더 선택",
    });
    if (typeof selected !== "string") return;
    if (target === "project") projectPath = selected;
    if (target === "save") saveAsPath = selected;
    if (target === "data") dataRoot = selected;
  }

  async function pickSchema() {
    const selected = await openDialog({
      multiple: false,
      directory: false,
      title: ".poly 스키마 파일 선택",
      filters: [{ name: "PolyGen schema", extensions: ["poly"] }],
    });
    if (typeof selected === "string") schemaPath = selected;
  }

  async function pickSources() {
    const selected = await openDialog({
      multiple: false,
      directory: false,
      title: ".sources.toml 파일 선택",
      filters: [{ name: "PolyGen sources", extensions: ["toml"] }],
    });
    if (typeof selected === "string") sourcesPath = selected;
  }

  function openProjectDialog(mode: "save" | "schema") {
    projectDialogMode = mode;
    projectDialogOpen = true;
    error = "";
    if (mode === "save") {
      saveAsPath = project?.root || "";
    }
  }

  async function completeProjectDialog() {
    if (projectDialogMode === "schema" && !schemaPath) {
      error = ".poly 스키마 파일을 선택해 주세요.";
      return;
    }
    if (projectDialogMode === "save" && !saveAsPath) {
      error = "저장할 PolySheet 프로젝트 폴더를 선택해 주세요.";
      return;
    }
    if (!(await syncGrid())) return;
    let summary = project!;
    if (schemaPath) {
      const attached = await run(() =>
        invoke<ProjectSummary>("attach_schema", {
          schema: schemaPath,
          sources: sourcesPath || null,
          dataRoot: dataRoot || null,
        }),
      );
      if (!attached) return;
      summary = attached;
      project = attached;
    }

    const shouldPersist = projectDialogMode === "save" || summary.isSaved;
    if (shouldPersist) {
      const approveNormalization = await requestNormalizationApproval(summary);
      if (approveNormalization === undefined) return;
      const persisted =
        projectDialogMode === "save"
          ? await run(() =>
              invoke<ProjectSummary>("save_project_as", {
                root: saveAsPath,
                approveNormalization,
              }),
            )
          : await run(() =>
              invoke<ProjectSummary>("save_project", {
                approveNormalization,
              }),
            );
      if (!persisted) return;
      summary = persisted;
    }

    project = summary;
    projectPath = summary.root;
    rememberRecentProject(summary);
    projectDialogOpen = false;
    await rebuildWorkbook();
    if (selectedSheetId) await selectSheet(selectedSheetId);
    status =
      projectDialogMode === "save"
        ? `프로젝트를 ${summary.root}에 저장했습니다.`
        : `${summary.schemaPath} 스키마를 연결했습니다.`;
  }

  async function bindSheet() {
    if (!bindDefinition) return;
    if (!(await syncGrid())) return;
    const table = project?.tables.find((entry) => entry.fqn === bindDefinition);
    const summary = await run(() =>
      invoke<ProjectSummary>("bind_data_sheet", {
        name: newSheetName || table?.name || "Data",
        definition: bindDefinition,
      }),
    );
    if (!summary) return;
    project = summary;
    mergeReport = null;
    selectedSheetId = summary.sheets.at(-1)?.id ?? "";
    newSheetName = "";
    await rebuildWorkbook();
    await selectSheet(selectedSheetId);
  }

  async function addCalculationSheet() {
    if (!(await syncGrid())) return;
    const summary = await run(() =>
      invoke<ProjectSummary>("add_calculation_sheet", {
        name: newSheetName || "Calculation",
      }),
    );
    if (!summary) return;
    project = summary;
    mergeReport = null;
    selectedSheetId = summary.sheets.at(-1)?.id ?? "";
    newSheetName = "";
    await rebuildWorkbook();
    await selectSheet(selectedSheetId);
  }

  async function loadRows(sheet: SheetSummary): Promise<RowRecord[]> {
    if (sheet.kind !== "data") return [];
    const rows: RowRecord[] = [];
    const chunkSize = 2_000;
    for (let offset = 0; offset < sheet.rowCount || offset === 0; offset += chunkSize) {
      const chunk = await invoke<RowChunk>("get_rows_chunk", {
        sheetId: sheet.id,
        offset,
        limit: chunkSize,
      });
      rows.push(...chunk.rows);
      if (rows.length >= chunk.total) break;
      await tick();
    }
    return rows;
  }

  async function rebuildWorkbook() {
    if (!project) return;
    await tick();
    if (!gridHost) {
      error = "스프레드시트 화면을 준비하지 못했습니다.";
      return;
    }
    selectionSubscriptions.forEach((subscription) => subscription.dispose());
    selectionSubscriptions = [];
    gridSyncTimers.forEach((timer) => clearTimeout(timer));
    gridSyncTimers.clear();
    gridSelectionCleanup?.();
    gridSelectionCleanup = null;
    installGridInputInspection(gridHost);
    workbook?.dispose();
    univer?.dispose();
    rowsBySheet = new Map();
    formulasBySheet = new Map();
    calculationsBySheet = new Map();

    const sheets: Record<string, Partial<IWorksheetData>> = {};
    const sheetOrder: string[] = [];
    for (const sheet of project.sheets) {
      const table = project.tables.find((entry) => entry.fqn === sheet.definition);
      const rows = await loadRows(sheet);
      rowsBySheet.set(sheet.id, rows);
      const fields = table?.fields ?? [];
      const cellData: Record<number, Record<number, ICellData>> = {};
      if (sheet.kind === "data") {
        const formulas = await invoke<FormulaDocument>("get_sheet_formulas", {
          sheetId: sheet.id,
        });
        formulasBySheet.set(sheet.id, formulas);
        cellData[0] = {
          0: {
            v: "__row_id",
            s: { bg: { rgb: "#172033" }, cl: { rgb: "#8da2c9" }, bl: 1 },
          },
        };
        fields.forEach((field, index) => {
          cellData[0][index + 1] = {
            v: field.name,
            custom: { polysheetFieldPath: field.name },
            s: { bg: { rgb: "#172033" }, cl: { rgb: "#f4f7fb" }, bl: 1 },
          };
        });
        rows.forEach((row, rowIndex) => {
          const target: Record<number, ICellData> = {
            0: { v: row.id },
          };
          fields.forEach((field, fieldIndex) => {
            const raw = row.value[field.name];
            const formula = findFormula(sheet.id, row.id, field.name);
            const custom = {
              polysheetRowId: row.id,
              polysheetFieldPath: field.name,
            };
            target[fieldIndex + 1] = formula
              ? { v: toGridFieldValue(raw, field), f: formula, custom }
              : { v: toGridFieldValue(raw, field), custom };
          });
          cellData[rowIndex + 1] = target;
        });
      } else {
        const calculation = await invoke<CalculationDocument>("get_calculation_sheet", {
          sheetId: sheet.id,
        });
        calculationsBySheet.set(sheet.id, calculation);
        calculation.row_order.forEach((rowId, rowIndex) => {
          calculation.column_order.forEach((columnId, columnIndex) => {
            const cell = calculation.cells[`${rowId}:${columnId}`];
            if (!cell) return;
            cellData[rowIndex] ??= {};
            cellData[rowIndex][columnIndex] = {
              v: toGridValue(cell.value),
              ...(cell.formula ? { f: cell.formula } : {}),
            };
          });
        });
      }
      sheets[sheet.id] = {
        id: sheet.id,
        name: sheet.name,
        rowCount: Math.max(rows.length + 100, 200),
        columnCount: Math.max(fields.length + 1, sheet.kind === "calculation" ? 26 : 8),
        cellData,
        columnData: sheet.kind === "data" ? { 0: { hd: 1 } } : {},
        freeze: { startRow: 1, ySplit: 1, startColumn: 0, xSplit: 0 },
      };
      sheetOrder.push(sheet.id);
    }
    if (sheetOrder.length === 0) {
      sheets["welcome"] = {
        id: "welcome",
        name: "Welcome",
        rowCount: 100,
        columnCount: 20,
        cellData: { 0: { 0: { v: "왼쪽에서 데이터 시트나 계산 시트를 추가하세요." } } },
      };
      sheetOrder.push("welcome");
    }

    const created = createUniver({
      locale: LocaleType.KO_KR,
      locales: {
        [LocaleType.KO_KR]: mergeLocales(koKR),
      },
      presets: [
        UniverSheetsCorePreset({
          ...POLYSHEET_UNIVER_UI_POLICY,
          container: gridHost,
          header: false,
        }),
        UniverSheetsDataValidationPreset({
          showEditOnDropdown: false,
          showSearchOnDropdown: true,
        }),
      ],
    });
    univer = created.univer;
    const api = created.univerAPI;
    univerAPI = api;
    workbook = api.createWorkbook({
      id: `polysheet-${Date.now()}`,
      name: project.name,
      appVersion: "0.1.0",
      locale: LocaleType.KO_KR,
      styles: {},
      sheetOrder,
      sheets,
    });
    if (selectedSheetId) {
      const sheet = workbook.getSheetBySheetId(selectedSheetId);
      if (sheet) workbook.setActiveSheet(sheet);
    }
    applyColumnValidation();
    subscribeUniverSelectionEvent(() =>
      api.addEvent(api.Event.BeforeCommandExecute, (event) => {
        if (!isBlockedUnmanagedCommand(event.id)) return;
        event.cancel = true;
        error =
          "시트 구조와 서식은 현재 PolySheet가 보존하는 전용 작업만 사용할 수 있습니다.";
      }),
    );
    subscribeUniverSelectionEvent(() =>
      api.addEvent(
        api.Event.SelectionChanged,
        ({ selections, worksheet }) => {
          const sheetId = worksheet.getSheetId();
          void handleGridSelection(
            selections.map((range) => ({ ...range, sheetId })),
            "SelectionChanged",
          );
        },
      ),
    );
    subscribeUniverSelectionEvent(() =>
      api.addEvent(
        api.Event.CellClicked,
        ({ row, column, worksheet }) => {
          void handleGridCell(worksheet.getSheetId(), row, column, "CellClicked");
        },
      ),
    );
    subscribeUniverSelectionEvent(() =>
      api.addEvent(
        api.Event.CellPointerUp,
        ({ row, column, worksheet }) => {
          void handleGridCell(worksheet.getSheetId(), row, column, "CellPointerUp");
        },
      ),
    );
    subscribeUniverSelectionEvent(() =>
      api.addEvent(
        api.Event.SheetEditEnded,
        ({ row, worksheet, isConfirm }) => {
          if (isConfirm) scheduleDataRowSync(worksheet.getSheetId(), row);
        },
      ),
    );
    subscribeUniverSelectionEvent(() =>
      api.addEvent(
        api.Event.CommandExecuted,
        ({ id, params }) => {
          if (id !== SetRangeValuesCommand.id) return;
          const sheetId = String(params?.subUnitId ?? "");
          const startRow = Number(params?.range?.startRow);
          const endRow = Number(params?.range?.endRow);
          if (!sheetId || !Number.isInteger(startRow) || !Number.isInteger(endRow)) {
            return;
          }
          for (let row = startRow; row <= endRow; row += 1) {
            scheduleDataRowSync(sheetId, row);
          }
        },
      ),
    );
  }

  function scheduleDataRowSync(sheetId: string, rowIndex: number) {
    const key = `${sheetId}:${rowIndex}`;
    const previous = gridSyncTimers.get(key);
    if (previous) clearTimeout(previous);
    gridSyncTimers.set(
      key,
      setTimeout(() => {
        gridSyncTimers.delete(key);
        void syncDataGridRow(sheetId, rowIndex);
      }, 25),
    );
  }

  async function syncDataGridRow(sheetId: string, rowIndex: number) {
    if (!project || !workbook || rowIndex < 1) return;
    const sheetSummary = project.sheets.find((sheet) => sheet.id === sheetId);
    const table = project.tables.find((entry) => entry.fqn === sheetSummary?.definition);
    const sheet = workbook.getSheetBySheetId(sheetId);
    const sourceRows = rowsBySheet.get(sheetId);
    if (!sheetSummary || sheetSummary.kind !== "data" || !table || !sheet || !sourceRows) {
      return;
    }

    try {
      await tick();
      const rowId = String(sheet.getRange(rowIndex, 0).getValue() ?? "");
      const source = sourceRows.find((row) => row.id === rowId);
      if (!rowId || !source) return;

      const edits: CellEditPayload[] = [];
      table.fields.forEach((field, fieldIndex) => {
        if (field.isPrimaryKey) return;
        const range = sheet.getRange(rowIndex, fieldIndex + 1);
        const formula = range.getFormulas()[0]?.[0] || undefined;
        const value = fromGridValue(range.getValue(), field);
        if (formula && typeof value === "string" && value.startsWith("#")) {
          throw new Error(`${field.name} 수식 오류: ${value}`);
        }
        const previousFormula = findFormula(sheetId, rowId, field.name);
        if (
          JSON.stringify(value) !== JSON.stringify(source.value[field.name]) ||
          formula !== previousFormula
        ) {
          edits.push({
            sheetId,
            rowId,
            fieldPath: field.name,
            value,
            formula,
          });
        }
      });
      if (edits.length === 0) return;

      const summary = await run(() =>
        invoke<ProjectSummary>("apply_cell_edits", {
          edits,
          calculationEdits: [],
        }),
      );
      if (!summary) return;
      project = summary;
      rememberGridEdits(sheetId, edits);
      mergeReport = null;

      if (inspectorMode === "whole" && selectedSheetId === sheetId) {
        await showWholeJson();
      } else if (
        inspectorSelection.kind === "row" &&
        inspectorSelection.rowId === rowId
      ) {
        await loadInspector(inspectorSelection, sheetId);
      }
      status = `${table.name} · ${edits.length}개 셀을 JSON 모델에 자동 반영했습니다.`;
    } catch (cause) {
      error = String(cause);
    }
  }

  function rememberGridEdits(
    sheetId: string,
    edits: CellEditPayload[],
  ) {
    const rows = rowsBySheet.get(sheetId) ?? [];
    const formulas = formulasBySheet.get(sheetId);
    for (const edit of edits) {
      const rowId = String(edit.rowId);
      const fieldPath = String(edit.fieldPath);
      const source = rows.find((row) => row.id === rowId);
      if (source) source.value[fieldPath] = edit.value;
      if (!formulas) continue;
      const formula =
        typeof edit.formula === "string" && edit.formula.trim()
          ? edit.formula
          : undefined;
      if (formula) {
        formulas.rows[rowId] ??= {};
        formulas.rows[rowId][fieldPath] = formula;
      } else if (formulas.rows[rowId]) {
        delete formulas.rows[rowId][fieldPath];
        if (Object.keys(formulas.rows[rowId]).length === 0) {
          delete formulas.rows[rowId];
        }
      }
    }
    rowsBySheet.set(sheetId, rows);
    if (formulas) {
      formulasBySheet.set(sheetId, formulas);
      formulasBySheet = new Map(formulasBySheet);
    }
  }

  function installGridInputInspection(host: HTMLDivElement) {
    let pendingLocation: { sheetId: string; rowIndex: number } | null = null;
    const captureActiveLocation = (event: PointerEvent) => {
      const bounds = host.getBoundingClientRect();
      const insideGrid =
        event.clientX >= bounds.left &&
        event.clientX <= bounds.right &&
        event.clientY >= bounds.top &&
        event.clientY <= bounds.bottom;
      if (insideGrid) pendingLocation = activeGridLocation();
    };
    const inspectSelection = (event: PointerEvent | KeyboardEvent) => {
      if (event instanceof PointerEvent) {
        const bounds = host.getBoundingClientRect();
        const insideGrid =
          event.clientX >= bounds.left &&
          event.clientX <= bounds.right &&
          event.clientY >= bounds.top &&
          event.clientY <= bounds.bottom;
        if (!insideGrid) return;
        selectionTrace = `DOM ${event.type} · x=${Math.round(event.clientX - bounds.left)}, y=${Math.round(event.clientY - bounds.top)}`;
      }
      const previous = pendingLocation;
      pendingLocation = null;
      window.setTimeout(() => {
        if (previous) {
          scheduleDataRowSync(previous.sheetId, previous.rowIndex);
        }
        const current = activeGridLocation();
        if (current) scheduleDataRowSync(current.sheetId, current.rowIndex);
        void inspectActiveGridSelection();
      }, 0);
    };
    window.addEventListener("pointerdown", captureActiveLocation, true);
    window.addEventListener("pointerup", inspectSelection, true);
    window.addEventListener("keyup", inspectSelection, true);
    gridSelectionCleanup = () => {
      window.removeEventListener("pointerdown", captureActiveLocation, true);
      window.removeEventListener("pointerup", inspectSelection, true);
      window.removeEventListener("keyup", inspectSelection, true);
    };
    selectionTrace = "DOM 선택 이벤트가 연결됨";
  }

  function activeGridLocation() {
    if (!workbook) return null;
    const sheetId = workbook.getActiveSheet().getSheetId();
    const range = workbook.getActiveRange()?.getRange();
    if (!range) return null;
    return { sheetId, rowIndex: range.startRow };
  }

  function subscribeUniverSelectionEvent(
    subscribe: () => { dispose(): void },
  ) {
    try {
      selectionSubscriptions.push(subscribe());
    } catch (cause) {
      selectionTrace = `Univer 이벤트 구독 실패 · ${String(cause)}`;
      console.error("PolySheet Univer selection subscription failed", cause);
    }
  }

  function findFormula(sheetId: string, rowId: string, field: string) {
    return formulasBySheet.get(sheetId)?.rows[rowId]?.[field] ?? "";
  }

  function toGridValue(
    value: unknown,
  ): string | number | boolean | null | undefined {
    if (value !== null && typeof value === "object") return JSON.stringify(value);
    if (
      value === null ||
      value === undefined ||
      typeof value === "string" ||
      typeof value === "number" ||
      typeof value === "boolean"
    ) {
      return value;
    }
    return String(value);
  }

  function toGridFieldValue(
    value: unknown,
    field: FieldSummary,
  ): string | number | boolean | null | undefined {
    if (value === null && field.isOptional && field.typeName.includes("string")) {
      return NULL_VALUE;
    }
    return toGridValue(value);
  }

  function fromGridValue(value: unknown, field: FieldSummary): unknown {
    if (value === NULL_VALUE && field.isOptional) return null;
    if (
      field.typeName.includes("string") &&
      value !== null &&
      value !== undefined
    ) {
      return String(value);
    }
    if (field.isList || field.isStruct) {
      if (typeof value !== "string") return value;
      return value.trim() ? JSON.parse(value) : field.isList ? [] : null;
    }
    if (value === null || value === undefined) {
      return field.isOptional ? null : field.typeName.includes("string") ? "" : value;
    }
    if (/^[iu](8|16|32)$/.test(field.typeName) && typeof value === "string") {
      return Number.parseInt(value, 10);
    }
    if (/^f(32|64)$/.test(field.typeName) && typeof value === "string") {
      return Number.parseFloat(value);
    }
    return value;
  }

  function applyColumnValidation() {
    if (!project || !workbook || !univerAPI) return;
    for (const sheetSummary of project.sheets) {
      if (sheetSummary.kind !== "data") continue;
      const table = project.tables.find((entry) => entry.fqn === sheetSummary.definition);
      const sheet = workbook.getSheetBySheetId(sheetSummary.id);
      if (!table || !sheet) continue;
      table.fields.forEach((field, index) => {
        let values = [...field.enumValues];
        if (field.foreignKeyTarget && field.foreignKeyField) {
          const targetSheet = project?.sheets.find(
            (candidate) => candidate.definition === field.foreignKeyTarget,
          );
          const targetRows = targetSheet ? rowsBySheet.get(targetSheet.id) ?? [] : [];
          values = targetRows
            .map((row) => row.value[field.foreignKeyField!])
            .filter((value) => value !== null && value !== undefined)
            .map(String);
        }
        if (field.isOptional && field.typeName.includes("string")) values.push(NULL_VALUE);
        values = [...new Set(values)];
        if (values.length === 0 || values.length > 5_000) return;
        const rule = univerAPI!
          .newDataValidation()
          .requireValueInList(values)
          .build();
        sheet
          .getRange(1, index + 1, Math.max(sheetSummary.rowCount + 100, 200), 1)
          .setDataValidation(rule);
      });
    }
  }

  async function selectSheet(id: string) {
    if (selectedSheetId && selectedSheetId !== id && !(await syncGrid())) return;
    selectedSheetId = id;
    const sheet = workbook?.getSheetBySheetId(id);
    if (sheet && workbook) workbook.setActiveSheet(sheet);
    const summary = project?.sheets.find((entry) => entry.id === id);
    if (summary?.kind === "data") {
      inspectorMode = "context";
      pendingInspectorSelection = null;
      await loadInspector({ kind: "create" }, id);
    } else {
      jsonText = "";
      inspectorBaseline = "";
      rowDraft = null;
      rowComparison = null;
    }
  }

  async function handleGridSelection(ranges: IRange[], source = "ActiveRange") {
    if (!project || ranges.length === 0) return;
    const rangeSheetId = ranges.find((range) => range.sheetId)?.sheetId ?? selectedSheetId;
    const summary = project.sheets.find((sheet) => sheet.id === rangeSheetId);
    if (!summary || summary.kind !== "data") return;
    if (selectedSheetId !== summary.id) {
      selectedSheetId = summary.id;
      rowDraft = null;
    }
    const table = project.tables.find((candidate) => candidate.fqn === summary.definition);
    const sheet = workbook?.getSheetBySheetId(summary.id);
    if (!table || !sheet) return;

    const idValues = sheet
      .getRange(1, 0, Math.max(summary.rowCount, 1), 1)
      .getValues() as unknown[][];
    const sourceRows = rowsBySheet.get(summary.id) ?? [];
    const rowIds = Array.from({ length: summary.rowCount }, (_, rowIndex) => {
      const cellData = sheet.getRange(rowIndex + 1, 1).getCellData();
      const customRowId = cellData?.custom?.polysheetRowId;
      if (typeof customRowId === "string" && customRowId) return customRowId;
      const hiddenRowId = String(idValues[rowIndex]?.[0] ?? "");
      return hiddenRowId || sourceRows[rowIndex]?.id || "";
    });
    const next = classifySelection(ranges, {
      fields: table.fields.map((field) => field.name),
      rowIds,
    });
    selectionTrace = `${source} · ${formatRanges(ranges)} · ${formatInspectorSelection(next)}`;
    inspectorMode = "context";
    if (sameInspectorSelection(inspectorSelection, next)) return;
    if (inspectorDirty && !sameInspectorSelection(inspectorSelection, next)) {
      pendingInspectorSelection = next;
      return;
    }
    await loadInspector(next, summary.id);
  }

  async function handleGridCell(
    sheetId: string,
    row: number,
    column: number,
    source: string,
  ) {
    if (!project) return;
    const summary = project.sheets.find((sheet) => sheet.id === sheetId);
    if (!summary || summary.kind !== "data") return;
    const table = project.tables.find((candidate) => candidate.fqn === summary.definition);
    const sheet = workbook?.getSheetBySheetId(summary.id);
    if (!table || !sheet) return;
    if (selectedSheetId !== summary.id) {
      selectedSheetId = summary.id;
      rowDraft = null;
    }
    const idValues = sheet
      .getRange(1, 0, Math.max(summary.rowCount, 1), 1)
      .getValues() as unknown[][];
    const sourceRows = rowsBySheet.get(summary.id) ?? [];
    const rowIds = Array.from({ length: summary.rowCount }, (_, rowIndex) => {
      const cellData = sheet.getRange(rowIndex + 1, 1).getCellData();
      const customRowId = cellData?.custom?.polysheetRowId;
      if (typeof customRowId === "string" && customRowId) return customRowId;
      const hiddenRowId = String(idValues[rowIndex]?.[0] ?? "");
      return hiddenRowId || sourceRows[rowIndex]?.id || "";
    });
    const next = classifyCell(row, column, {
      fields: table.fields.map((field) => field.name),
      rowIds,
    });
    selectionTrace = `${source} · row=${row}, col=${column} · ${formatInspectorSelection(next)}`;
    inspectorMode = "context";
    if (sameInspectorSelection(inspectorSelection, next)) return;
    if (inspectorDirty) {
      pendingInspectorSelection = next;
      return;
    }
    await loadInspector(next, summary.id);
  }

  async function inspectActiveGridSelection() {
    if (!workbook) return;
    const activeSheet = workbook.getActiveSheet();
    const sheetId = activeSheet.getSheetId();
    const activeRange = workbook.getActiveRange();
    const ranges = activeRange ? [{ ...activeRange.getRange(), sheetId }] : [];
    if (ranges.length > 0) await handleGridSelection(ranges, "ActiveRange");
  }

  function formatRanges(ranges: IRange[]) {
    return ranges
      .map(
        (range) =>
          `r${range.startRow}:${range.endRow},c${range.startColumn}:${range.endColumn},t${range.rangeType}`,
      )
      .join(" | ");
  }

  function formatInspectorSelection(selection: InspectorSelection) {
    if (selection.kind === "row") {
      return `행 ${selection.rowId} / ${selection.fieldPath ?? "전체"}`;
    }
    if (selection.kind === "field") return `필드 ${selection.fieldPath}`;
    if (selection.kind === "compare") return `${selection.rowIds.length}행 비교`;
    if (selection.kind === "compareLimit") return `${selection.count}행 선택(제한 초과)`;
    return "새 데이터";
  }

  function sameInspectorSelection(left: InspectorSelection, right: InspectorSelection) {
    return JSON.stringify(left) === JSON.stringify(right);
  }

  async function ensureRowDraft(sheetId: string) {
    rowDraft =
      (await run(() => invoke<RowDraft>("get_row_draft", { sheetId }))) ?? null;
    return rowDraft;
  }

  async function loadInspector(selection: InspectorSelection, sheetId = selectedSheetId) {
    inspectorSelection = selection;
    pendingInspectorSelection = null;
    rowComparison = null;
    if (selection.kind === "field") {
      if (!rowDraft) await ensureRowDraft(sheetId);
      jsonText = "";
      inspectorBaseline = "";
      return;
    }
    if (selection.kind === "row") {
      const text =
        (await run(() =>
          invoke<string>("get_row_json", {
            sheetId,
            rowId: selection.rowId,
          }),
        )) ?? "";
      jsonText = text;
      inspectorBaseline = text;
      return;
    }
    if (selection.kind === "compare") {
      rowComparison =
        (await run(() =>
          invoke<RowComparison>("compare_rows", {
            sheetId,
            rowIds: selection.rowIds,
          }),
        )) ?? null;
      jsonText = "";
      inspectorBaseline = "";
      return;
    }
    if (selection.kind === "compareLimit") {
      jsonText = "";
      inspectorBaseline = "";
      return;
    }
    const draft = await ensureRowDraft(sheetId);
    jsonText = draft?.json ?? "";
    inspectorBaseline = jsonText;
  }

  async function showWholeJson() {
    if (!selectedSheet || selectedSheet.kind !== "data") return;
    if (inspectorDirty) {
      status = "먼저 현재 JSON 편집을 적용하거나 취소해 주세요.";
      return;
    }
    inspectorMode = "whole";
    const text =
      (await run(() => invoke<string>("get_sheet_json", { sheetId: selectedSheet.id }))) ?? "";
    jsonText = text;
    inspectorBaseline = text;
  }

  async function discardInspectorEdit() {
    const next = pendingInspectorSelection ?? inspectorSelection;
    await loadInspector(next);
  }

  function draftFieldOptions(field: InspectorField): unknown[] {
    if (field.enumValues.length > 0) return field.enumValues;
    if (!field.foreignKeyTarget || !field.foreignKeyField || !project) return [];
    const targetSheet = project.sheets.find(
      (sheet) => sheet.definition === field.foreignKeyTarget,
    );
    return targetSheet
      ? [
          ...new Set(
            (rowsBySheet.get(targetSheet.id) ?? [])
              .map((row) => row.value[field.foreignKeyField!])
              .filter((value) => value !== null && value !== undefined)
              .map((value) => JSON.stringify(value)),
          ),
        ].map((value) => JSON.parse(value))
      : [];
  }

  function setDraftField(path: string, value: unknown) {
    try {
      const root = JSON.parse(jsonText || "{}") as Record<string, unknown>;
      const segments = path.split(".");
      let target = root;
      for (const segment of segments.slice(0, -1)) {
        const child = target[segment];
        if (!child || typeof child !== "object" || Array.isArray(child)) {
          target[segment] = {};
        }
        target = target[segment] as Record<string, unknown>;
      }
      target[segments.at(-1)!] = value;
      jsonText = `${JSON.stringify(root, null, 2)}\n`;
    } catch (cause) {
      error = `JSON 초안을 먼저 올바르게 고쳐 주세요: ${cause}`;
    }
  }

  function selectJsonCell(
    event: CustomEvent<{ rowIndex?: number; field: string }>,
  ) {
    if (
      !selectedSheet ||
      selectedSheet.kind !== "data" ||
      !selectedTable ||
      !workbook
    ) {
      return;
    }
    const fieldIndex = selectedTable.fields.findIndex(
      (field) => field.name === event.detail.field,
    );
    const selectedRowId =
      inspectorSelection.kind === "row" ? inspectorSelection.rowId : undefined;
    const row =
      inspectorMode === "context" && selectedRowId
        ? rowsBySheet
            .get(selectedSheet.id)
            ?.find((candidate) => candidate.id === selectedRowId)
        : rowsBySheet.get(selectedSheet.id)?.[event.detail.rowIndex ?? -1];
    const sheet = workbook.getSheetBySheetId(selectedSheet.id);
    if (fieldIndex < 0 || !row || !sheet) return;

    const idValues = sheet
      .getRange(1, 0, Math.max(selectedSheet.rowCount, 1), 1)
      .getValues();
    const visibleRowIndex = idValues.findIndex(
      (values) => String(values?.[0] ?? "") === row.id,
    );
    if (visibleRowIndex < 0) return;

    workbook.setActiveSheet(sheet);
    sheet.getRange(visibleRowIndex + 1, fieldIndex + 1).activate();
    status = `${selectedSheet.name} · ${event.detail.field} 셀을 선택했습니다.`;
  }

  async function syncGrid(): Promise<boolean> {
    if (!project || !workbook) return true;

    gridSyncTimers.forEach((timer) => clearTimeout(timer));
    gridSyncTimers.clear();
    const synchronized = await run(async () => {
      const dataEdits = collectDataGridEdits();
      const calculationUpdates = collectChangedCalculationUpdates(
        project!.sheets,
        calculationsBySheet,
        calculationDocumentFromGrid,
      );

      if (dataEdits.length > 0 || calculationUpdates.length > 0) {
        project = await invoke<ProjectSummary>("apply_cell_edits", {
          edits: dataEdits,
          calculationEdits: calculationUpdates.map((update) => ({
            sheetId: update.sheet.id,
            document: update.document,
          })),
        });
        for (const sheetId of new Set(dataEdits.map((edit) => edit.sheetId))) {
          rememberGridEdits(
            sheetId,
            dataEdits.filter((edit) => edit.sheetId === sheetId),
          );
        }
        for (const update of calculationUpdates) {
          calculationsBySheet.set(update.sheet.id, update.document);
        }
      }

      if (dataEdits.length > 0 || calculationUpdates.length > 0) {
        mergeReport = null;
      }
      return true;
    });
    return synchronized === true;
  }

  function collectDataGridEdits(): CellEditPayload[] {
    if (!project || !workbook) return [];
    const edits: CellEditPayload[] = [];
    for (const sheetSummary of project.sheets) {
      if (sheetSummary.kind !== "data") continue;
      const table = project.tables.find(
        (candidate) => candidate.fqn === sheetSummary.definition,
      );
      const sheet = workbook.getSheetBySheetId(sheetSummary.id);
      const original = rowsBySheet.get(sheetSummary.id) ?? [];
      if (!table || !sheet) continue;

      const range = sheet.getDataRange();
      const values = range.getValues();
      const formulas = range.getFormulas();
      for (let rowIndex = 1; rowIndex < values.length; rowIndex += 1) {
        const rowId = String(values[rowIndex]?.[0] ?? "");
        const source = original.find((row) => row.id === rowId);
        if (!source) continue;
        table.fields.forEach((field, fieldIndex) => {
          if (field.isPrimaryKey) return;
          const formula = formulas[rowIndex]?.[fieldIndex + 1] || undefined;
          const value = fromGridValue(values[rowIndex]?.[fieldIndex + 1], field);
          if (formula && typeof value === "string" && value.startsWith("#")) {
            throw new Error(`${field.name} 수식 오류: ${value}`);
          }
          if (
            JSON.stringify(value) !== JSON.stringify(source.value[field.name]) ||
            formula !== findFormula(sheetSummary.id, rowId, field.name)
          ) {
            edits.push({
              sheetId: sheetSummary.id,
              rowId,
              fieldPath: field.name,
              value,
              formula,
            });
          }
        });
      }
    }
    return edits;
  }

  function calculationDocumentFromGrid(sheetSummary: SheetSummary): CalculationDocument {
    if (!workbook) throw new Error("스프레드시트가 준비되지 않았습니다.");
    const sheet = workbook.getSheetBySheetId(sheetSummary.id);
    const current = calculationsBySheet.get(sheetSummary.id);
    if (!sheet || !current) {
      throw new Error(`계산 시트 '${sheetSummary.name}'을 동기화할 수 없습니다.`);
    }
    const values =
      current.row_order.length > 0 && current.column_order.length > 0
        ? sheet
            .getRange(
              0,
              0,
              current.row_order.length,
              current.column_order.length,
            )
            .getValues()
        : [];
    const formulas =
      current.row_order.length > 0 && current.column_order.length > 0
        ? sheet
            .getRange(
              0,
              0,
              current.row_order.length,
              current.column_order.length,
            )
            .getFormulas()
        : [];
    const cells: Record<string, CalculationCell> = {};
    current.row_order.forEach((rowId, rowIndex) => {
      current.column_order.forEach((columnId, columnIndex) => {
        const value = values[rowIndex]?.[columnIndex];
        const formula = formulas[rowIndex]?.[columnIndex] || undefined;
        if (formula && typeof value === "string" && value.startsWith("#")) {
          throw new Error(`${sheetSummary.name} 수식 오류: ${value}`);
        }
        if (formula || (value !== null && value !== undefined && value !== "")) {
          cells[`${rowId}:${columnId}`] = { value, formula };
        }
      });
    });
    return {
      ...current,
      needs_recalculation: false,
      cells,
    };
  }

  async function insertCalculationAxis(axis: "row" | "column") {
    if (!selectedSheet || selectedSheet.kind !== "calculation") return;
    try {
      if (!(await syncGrid())) return;
      const current = calculationsBySheet.get(selectedSheet.id);
      const activeRange = workbook?.getActiveRange();
      const index =
        axis === "row"
          ? (activeRange?.getRow() ?? current?.row_order.length ?? 0)
          : (activeRange?.getColumn() ?? current?.column_order.length ?? 0);
      const command =
        axis === "row" ? "insert_calculation_row" : "insert_calculation_column";
      const summary = await run(() =>
        invoke<ProjectSummary>(command, {
          sheetId: selectedSheet.id,
          index,
        }),
      );
      if (!summary) return;
      project = summary;
      await rebuildWorkbook();
      await selectSheet(selectedSheet.id);
      status =
        axis === "row"
          ? "선택한 위치에 stable ID 행을 추가했습니다."
          : "선택한 위치에 stable ID 열을 추가했습니다.";
    } catch (cause) {
      error = String(cause);
    }
  }

  async function applyJson() {
    if (!selectedSheet || selectedSheet.kind !== "data") return;
    jsonText = jsonEditor?.getValue() ?? jsonText;
    if (!(await syncGrid())) return;
    const sheetId = selectedSheet.id;
    let summary: ProjectSummary | undefined;
    let targetRowId: string | undefined;
    const targetFieldPath =
      inspectorSelection.kind === "row" ? inspectorSelection.fieldPath : undefined;
    if (inspectorMode === "whole") {
      summary = await run(() =>
        invoke<ProjectSummary>("apply_sheet_json", {
          sheetId,
          json: jsonText,
        }),
      );
      status = "전체 JSON을 검증하고 시트에 적용했습니다.";
    } else if (inspectorSelection.kind === "row") {
      targetRowId = inspectorSelection.rowId;
      summary = await run(() =>
        invoke<ProjectSummary>("apply_row_json", {
          sheetId,
          rowId: targetRowId,
          json: jsonText,
        }),
      );
      if (!summary) return;
      project = summary;
      mergeReport = null;
      const canonicalRow =
        (await run(() =>
          invoke<string>("get_row_json", {
            sheetId,
            rowId: targetRowId,
          }),
        )) ?? jsonText;
      refreshDataGridRow(sheetId, targetRowId, canonicalRow);
      inspectorBaseline = jsonText;
      pendingInspectorSelection = null;
      status = "선택한 행 JSON을 검증하고 적용했습니다.";
      return;
    } else if (inspectorSelection.kind === "create") {
      const inserted = await run(() =>
        invoke<InsertedRow>("insert_row_json", {
          sheetId,
          json: jsonText,
        }),
      );
      if (!inserted) return;
      summary = inserted.project;
      targetRowId = inserted.rowId;
      status = "새 행을 JSON 원본 순서의 마지막에 추가했습니다.";
    } else {
      return;
    }
    if (!summary) return;
    project = summary;
    mergeReport = null;
    inspectorBaseline = jsonText;
    pendingInspectorSelection = null;
    await tick();
    await rebuildWorkbook();
    await tick();
    if (targetRowId) {
      await activateRow(targetRowId, targetFieldPath);
      await loadInspector({ kind: "row", rowId: targetRowId, fieldPath: targetFieldPath });
    } else {
      await showWholeJson();
    }
  }

  function refreshDataGridRow(sheetId: string, rowId: string, rowJson: string) {
    if (!project || !workbook) return;
    const sheetSummary = project.sheets.find((sheet) => sheet.id === sheetId);
    const table = project.tables.find((entry) => entry.fqn === sheetSummary?.definition);
    const sheet = workbook.getSheetBySheetId(sheetId);
    const rows = rowsBySheet.get(sheetId);
    if (!sheetSummary || !table || !sheet || !rows) return;

    const sourceIndex = rows.findIndex((row) => row.id === rowId);
    if (sourceIndex < 0) return;
    const value = JSON.parse(rowJson) as Record<string, unknown>;
    rows[sourceIndex] = { id: rowId, value };
    rowsBySheet.set(sheetId, rows);

    const idValues = sheet
      .getRange(1, 0, Math.max(sheetSummary.rowCount, 1), 1)
      .getValues() as unknown[][];
    const visibleRowIndex = idValues.findIndex(
      (values) => String(values?.[0] ?? "") === rowId,
    );
    if (visibleRowIndex < 0) return;

    table.fields.forEach((field, fieldIndex) => {
      const formula = findFormula(sheetId, rowId, field.name);
      sheet.getRange(visibleRowIndex + 1, fieldIndex + 1).setValue({
        v: toGridFieldValue(value[field.name], field),
        ...(formula ? { f: formula } : {}),
        custom: {
          polysheetRowId: rowId,
          polysheetFieldPath: field.name,
        },
      });
    });
  }

  async function activateRow(rowId: string, fieldPath?: string) {
    if (!selectedSheet || !selectedTable || !workbook) return;
    const sheet = workbook.getSheetBySheetId(selectedSheet.id);
    if (!sheet) return;
    const idValues = sheet
      .getRange(1, 0, Math.max(selectedSheet.rowCount, 1), 1)
      .getValues() as unknown[][];
    const visibleRowIndex = idValues.findIndex(
      (values) => String(values?.[0] ?? "") === rowId,
    );
    if (visibleRowIndex < 0) return;
    const fieldIndex = Math.max(
      0,
      selectedTable.fields.findIndex((field) => field.name === fieldPath),
    );
    sheet.getRange(visibleRowIndex + 1, fieldIndex + 1).activate();
  }

  async function applyVisibleOrder() {
    if (!selectedSheet || selectedSheet.kind !== "data" || !workbook) return;
    if (!(await syncGrid())) return;
    const sheet = workbook.getSheetBySheetId(selectedSheet.id);
    if (!sheet) return;
    const values = sheet.getDataRange().getValues() as unknown[][];
    const rowIds: string[] = values
      .slice(1)
      .map((row: unknown[]) => String(row?.[0] ?? ""))
      .filter((id: string) => id.length > 0);
    const summary = await run(() =>
      invoke<ProjectSummary>("apply_row_order", {
        sheetId: selectedSheet.id,
        rowIds,
      }),
    );
    if (!summary) return;
    project = summary;
    mergeReport = null;
    rowsBySheet.set(
      selectedSheet.id,
      rowIds
        .map((id: string) =>
          rowsBySheet.get(selectedSheet.id)?.find((row: RowRecord) => row.id === id),
        )
        .filter((row): row is RowRecord => !!row),
    );
    if (inspectorMode === "whole") await showWholeJson();
    status = "현재 보이는 행 순서를 JSON 순서에 적용했습니다.";
  }

  async function saveProject() {
    if (!(await syncGrid())) {
      status = "화면 변경을 모델에 반영하지 못해 저장을 중단했습니다.";
      return;
    }
    if (!project?.isSaved) {
      openProjectDialog("save");
      status = "처음 저장할 프로젝트 폴더를 선택해 주세요.";
      return;
    }
    const approve = await requestNormalizationApproval(project);
    if (approve === undefined) return;
    const summary = await run(() =>
      invoke<ProjectSummary>("save_project", { approveNormalization: approve }),
    );
    if (!summary) return;
    project = summary;
    rememberRecentProject(summary);
    status = "JSON과 PolySheet sidecar를 저장했습니다.";
  }

  async function requestNormalizationApproval(
    summary: ProjectSummary,
  ): Promise<boolean | undefined> {
    if (summary.normalizationRequired.length === 0) return false;
    const preview = await run(() =>
      invoke<NormalizationPreview[]>("get_normalization_preview"),
    );
    if (!preview) return undefined;
    normalizationPreview = preview;
    rightTab = "normalization";
    const approve = window.confirm(
      `최초 저장은 ${preview.length}개 JSON 파일을 정규화합니다.\n` +
        `오른쪽의 정규화 미리보기를 확인했나요?\n\n` +
        `이 변경은 별도 커밋으로 저장하는 것을 권장합니다.`,
    );
    if (!approve) {
      projectDialogOpen = false;
      status = "오른쪽 정규화 미리보기를 확인한 뒤 저장을 다시 실행해 주세요.";
      return undefined;
    }
    return true;
  }

  async function showDiff() {
    if (!(await syncGrid())) return;
    const report = await run(() =>
      invoke<DiffReport>("diff_project", {
        base: diffBase,
        target: diffTarget,
      }),
    );
    if (!report) return;
    diffReport = report;
    rightTab = "diff";
    status = `${report.changes.length}개 의미 변경`;
  }

  async function stageSelected() {
    if (!selectedSheetId) return;
    if (!(await syncGrid())) return;
    const staged = await run(() =>
      invoke<string[]>("stage_sheets", { sheetIds: [selectedSheetId] }),
    );
    if (staged) status = `${staged.length}개 관련 파일을 Stage했습니다.`;
  }

  async function previewMerge() {
    if (!(await syncGrid())) return;
    const report = await run(() =>
      invoke<MergeReport>("preview_merge", {
        base: mergeBase,
        ours: mergeOurs,
        theirs: mergeTheirs,
      }),
    );
    if (!report) return;
    mergeReport = report;
    rightTab = "merge";
    status =
      report.conflicts.length > 0
        ? `${report.conflicts.length}개 병합 충돌`
        : report.recalculation_required
          ? "자동 병합 가능 — 적용 후 수식 재계산 필요"
          : "자동 병합할 수 있습니다.";
  }

  async function applyMerge() {
    if (!mergeReport || mergeReport.conflicts.length > 0) return;
    if (!(await syncGrid())) return;
    if (!mergeReport) {
      status = "편집 내용이 변경되어 병합 미리보기를 다시 실행해 주세요.";
      return;
    }
    const report = mergeReport;
    const summary = await run(() =>
      invoke<ProjectSummary>("apply_pending_merge"),
    );
    if (!summary) return;
    project = summary;
    mergeReport = null;
    await rebuildWorkbook();
    if (selectedSheetId) await selectSheet(selectedSheetId);
    status = report.recalculation_required
      ? "병합을 모델에 적용했습니다. 수식을 재계산한 뒤 저장하세요."
      : "병합 결과를 working tree에 원자적으로 저장했습니다.";
  }

  function identityRowId(issue: Diagnostic): string | null {
    return issue.message.match(/row id ([0-9a-f-]{36})/i)?.[1] ?? null;
  }

  async function confirmIdentity(issue: Diagnostic, reuseExisting: boolean) {
    const currentRowId = identityRowId(issue);
    const sheet = project?.sheets.find(
      (candidate) => candidate.kind === "data" && candidate.name === issue.path,
    );
    if (!currentRowId || !sheet) return;
    const replacementRowId = reuseExisting
      ? window.prompt("재연결할 기존 UUIDv7 row ID를 입력하세요.", currentRowId)
      : currentRowId;
    if (!replacementRowId) return;
    if (!(await syncGrid())) return;
    const summary = await run(() =>
      invoke<ProjectSummary>("confirm_row_identity", {
        sheetId: sheet.id,
        currentRowId,
        replacementRowId,
      }),
    );
    if (!summary) return;
    project = summary;
    await rebuildWorkbook();
    await selectSheet(sheet.id);
    status = reuseExisting
      ? "기존 행 ID에 재연결했습니다."
      : "이 행을 새 행 ID로 확인했습니다.";
  }

  onMount(() => {
    try {
      const stored = JSON.parse(
        localStorage.getItem(RECENT_PROJECTS_KEY) ?? "[]",
      ) as RecentProject[];
      recentProjects = stored
        .filter(
          (recent) =>
            typeof recent?.name === "string" &&
            typeof recent?.path === "string" &&
            recent.path.length > 0,
        )
        .slice(0, 8);
    } catch {
      recentProjects = [];
      localStorage.removeItem(RECENT_PROJECTS_KEY);
    }
  });

  onDestroy(() => {
    selectionSubscriptions.forEach((subscription) => subscription.dispose());
    gridSyncTimers.forEach((timer) => clearTimeout(timer));
    gridSyncTimers.clear();
    gridSelectionCleanup?.();
    workbook?.dispose();
    univer?.dispose();
  });
</script>

<svelte:head><title>{project ? `${project.name} — PolySheet` : "PolySheet"}</title></svelte:head>

{#if !project}
  <main class="welcome">
    <section class="welcome-card">
      <div class="brand-mark">PS</div>
      <p class="eyebrow">POLYGEN DATA STUDIO</p>
      <h1>데이터도 코드처럼<br />검토하고 병합하세요.</h1>
      <p class="welcome-copy">
        `.poly` 정의와 JSON을 연결하고, 스프레드시트 편집·수식·의미 기반 Diff를
        하나의 Git 친화적 작업공간에서 다룹니다.
      </p>
      {#if createMode}
        <label>
          <span>프로젝트 이름</span>
          <input bind:value={projectName} placeholder="Game Data" />
        </label>
        <p class="draft-note">
          경로와 스키마는 지금 정하지 않아도 됩니다. 빈 계산 시트에서 시작하고
          첫 저장 때 프로젝트 폴더를 선택하세요.
        </p>
        <button
          class="primary large"
          onclick={createProject}
          disabled={busy}
        >
          {busy ? "만드는 중…" : "빈 문서 시작"}
        </button>
      {:else}
        <label>
          <span>PolySheet 프로젝트 폴더</span>
          <div class="path-input">
            <input bind:value={projectPath} placeholder="D:\project\game.polysheet" />
            <button onclick={() => pickDirectory("project")}>찾아보기</button>
          </div>
        </label>
        <button class="primary large" onclick={openProject} disabled={busy || !projectPath}>
          {busy ? "여는 중…" : "프로젝트 열기"}
        </button>
        {#if recentProjects.length > 0}
          <section class="recent-projects">
            <div class="recent-heading">
              <strong>최근 프로젝트</strong>
              <span>{recentProjects.length}</span>
            </div>
            {#each recentProjects as recent}
              <div class="recent-project">
                <button
                  class="recent-open"
                  onclick={() => openRecentProject(recent.path)}
                  disabled={busy}
                >
                  <strong>{recent.name}</strong>
                  <span>{recent.path}</span>
                </button>
                <button
                  class="recent-remove"
                  title="최근 목록에서 제거"
                  aria-label={`${recent.name} 최근 목록에서 제거`}
                  onclick={() => removeRecentProject(recent.path)}
                >×</button>
              </div>
            {/each}
          </section>
        {/if}
      {/if}
      <button class="text-button" onclick={() => (createMode = !createMode)}>
        {createMode ? "기존 프로젝트 열기" : "새 프로젝트 만들기"}
      </button>
      {#if error}<p class="error">{error}</p>{/if}
    </section>
  </main>
{:else}
  <div class="app-shell">
    <header class="topbar">
      <div class="brand"><span>PS</span><strong>{project.name}</strong></div>
      <div class="project-path">
        {project.schemaPath || "스키마 미연결"} · {project.isSaved ? project.root : "저장되지 않음"}
      </div>
      <div class="toolbar">
        {#if !project.schemaPath}
          <button onclick={() => openProjectDialog("schema")} disabled={busy}>스키마 연결</button>
        {/if}
        {#if selectedSheet?.kind === "calculation"}
          <button onclick={() => insertCalculationAxis("row")} disabled={busy}>행 삽입</button>
          <button onclick={() => insertCalculationAxis("column")} disabled={busy}>열 삽입</button>
        {/if}
        <button onclick={applyVisibleOrder} disabled={busy || selectedSheet?.kind !== "data"}>
          현재 순서 적용
        </button>
        <button class="primary" onclick={saveProject} disabled={busy}>저장</button>
        <button onclick={showDiff} disabled={busy || !project.isSaved}>Diff</button>
        <button onclick={stageSelected} disabled={busy || !project.isSaved || !selectedSheetId}>
          Stage
        </button>
      </div>
    </header>

    <aside class="sidebar">
      <div class="sidebar-heading">
        <span>시트</span>
        <span class="count">{project.sheets.length}</span>
      </div>
      <nav>
        {#each project.sheets as sheet}
          <button
            class:active={sheet.id === selectedSheetId}
            onclick={() => selectSheet(sheet.id)}
          >
            <span class="sheet-icon">{sheet.kind === "data" ? "D" : "ƒ"}</span>
            <span>
              <strong>{sheet.name}</strong>
              <small>{sheet.definition ?? "계산 시트"} · {sheet.rowCount.toLocaleString()} rows</small>
            </span>
          </button>
        {/each}
      </nav>

      <div class="add-sheet">
        <input bind:value={newSheetName} placeholder="새 시트 이름" />
        <select bind:value={bindDefinition}>
          <option value="">테이블 정의 선택</option>
          {#each project.tables.filter((table) => !table.isReadonly && table.jsonSource) as table}
            <option value={table.fqn}>{table.fqn}</option>
          {/each}
        </select>
        <div class="add-actions">
          <button onclick={bindSheet} disabled={!bindDefinition}>데이터 시트</button>
          <button onclick={addCalculationSheet}>계산 시트</button>
        </div>
      </div>

      <div class="git-box">
        {#if project.isSaved}
          <span>Git 비교</span>
          <select bind:value={diffBase}>
            <option value="HEAD">HEAD</option>
            {#each gitRefs as reference}<option value={reference}>{reference}</option>{/each}
          </select>
          <select bind:value={diffTarget}>
            <option value="WORKTREE">Working tree</option>
            {#each gitRefs as reference}<option value={reference}>{reference}</option>{/each}
          </select>
          <button onclick={showDiff}>비교</button>
          <span>3-way merge</span>
          <input bind:value={mergeBase} placeholder="base ref" />
          <select bind:value={mergeOurs}>
            <option value="WORKTREE">Working tree (ours)</option>
            {#each gitRefs as reference}<option value={reference}>{reference}</option>{/each}
          </select>
          <select bind:value={mergeTheirs}>
            {#each gitRefs as reference}<option value={reference}>{reference}</option>{/each}
          </select>
          <button onclick={previewMerge}>병합 미리보기</button>
        {:else}
          <span>Git 기능은 프로젝트를 처음 저장한 뒤 사용할 수 있습니다.</span>
        {/if}
      </div>
    </aside>

    <section class="workspace">
      <div class="sheet-context">
        <div>
          <strong>{selectedSheet?.name ?? "시트 없음"}</strong>
          <span>{selectedSheet?.definition ?? "자유 계산"}</span>
        </div>
        {#if selectedTable}
          <div class="field-pills">
            {#each selectedTable.fields.slice(0, 6) as field}
              <span class:pk={field.isPrimaryKey}>{field.name} · {field.typeName}</span>
            {/each}
          </div>
        {/if}
      </div>
      <div class="grid-host" bind:this={gridHost}></div>
    </section>

    <aside class="inspector">
      <div class="tabs">
        <button class:active={rightTab === "json"} onclick={() => (rightTab = "json")}>JSON</button>
        <button class:active={rightTab === "issues"} onclick={() => (rightTab = "issues")}>
          검증 <span>{project.diagnostics.length}</span>
        </button>
        <button class:active={rightTab === "diff"} onclick={() => (rightTab = "diff")}>
          Diff <span>{diffReport.changes.length}</span>
        </button>
        <button class:active={rightTab === "merge"} onclick={() => (rightTab = "merge")}>
          Merge <span>{mergeReport?.conflicts.length ?? 0}</span>
        </button>
        {#if normalizationPreview.length > 0}
          <button
            class:active={rightTab === "normalization"}
            onclick={() => (rightTab = "normalization")}
          >
            정규화 <span>{normalizationPreview.length}</span>
          </button>
        {/if}
      </div>
      <div class="inspector-body">
        {#if rightTab === "json"}
          {#if selectedSheet?.kind === "data"}
            <div class="inspector-switcher">
              <span class:active={inspectorMode === "context"}>
                {inspectorSelection.kind === "field"
                  ? "필드"
                  : inspectorSelection.kind === "row"
                    ? "행 JSON"
                    : inspectorSelection.kind === "compare" ||
                        inspectorSelection.kind === "compareLimit"
                      ? "행 비교"
                      : "새 데이터"}
              </span>
              <button
                class:active={inspectorMode === "whole"}
                onclick={showWholeJson}
              >전체 JSON</button>
            </div>
            <div class="selection-trace">{selectionTrace}</div>
            {#if pendingInspectorSelection}
              <div class="pending-edit">
                <span>현재 JSON에 오류가 있어 기존 행에 고정했습니다.</span>
                <button onclick={discardInspectorEdit}>변경 취소</button>
              </div>
            {/if}
            {#if inspectorMode === "whole"}
              <div class="json-editor-wrap">
                {#if inspectorFormulaHints.length > 0}
                  <div class="formula-hint-list">
                    {#each inspectorFormulaHints as hint}
                      <div>
                        <span>{hint.path.join(".")}</span>
                        <code>{inlineJsonValue(hint.value)}</code>
                        <strong>ƒ {hint.formula}</strong>
                      </div>
                    {/each}
                  </div>
                {/if}
                <JsonEditor
                  bind:this={jsonEditor}
                  value={jsonText}
                  formulaHints={inspectorFormulaHints}
                  on:change={(event) => (jsonText = event.detail)}
                  on:commit={(event) => {
                    jsonText = event.detail;
                    void applyJson();
                  }}
                  on:select={selectJsonCell}
                />
                <span class="auto-apply-note">값을 벗어나면 자동 검증·적용</span>
              </div>
            {:else if inspectorSelection.kind === "field"}
              {#if selectedInspectorField}
                <article class="field-definition">
                  <div class="definition-heading">
                    <div>
                      <span>FIELD</span>
                      <h3>{selectedInspectorField.path}</h3>
                    </div>
                    <code>{selectedInspectorField.typeName}</code>
                  </div>
                  <div class="definition-badges">
                    <span>{selectedInspectorField.required ? "필수" : "Optional · null 가능"}</span>
                    {#if selectedInspectorField.isPrimaryKey}<span class="warning">Primary key · 직접 수정 불가</span>{/if}
                    {#if selectedInspectorField.isList}<span>List · 배열 전체 비교</span>{/if}
                    {#if selectedInspectorField.isStruct}<span>중첩 데이터</span>{/if}
                  </div>
                  <dl>
                    {#if selectedInspectorField.defaultValue !== undefined}
                      <dt>기본값</dt><dd>{selectedInspectorField.defaultValue}</dd>
                    {/if}
                    {#if selectedInspectorField.enumValues.length}
                      <dt>선택 값</dt><dd>{selectedInspectorField.enumValues.join(", ")}</dd>
                    {/if}
                    {#if selectedInspectorField.foreignKeyTarget}
                      <dt>참조</dt><dd>{selectedInspectorField.foreignKeyTarget}.{selectedInspectorField.foreignKeyField}</dd>
                    {/if}
                    {#if selectedInspectorField.maxLength}
                      <dt>최대 길이</dt><dd>{selectedInspectorField.maxLength}</dd>
                    {/if}
                    {#if selectedInspectorField.rangeMin || selectedInspectorField.rangeMax}
                      <dt>범위</dt><dd>{selectedInspectorField.rangeMin} ~ {selectedInspectorField.rangeMax}</dd>
                    {/if}
                    {#if selectedInspectorField.regexPattern}
                      <dt>패턴</dt><dd><code>{selectedInspectorField.regexPattern}</code></dd>
                    {/if}
                    <dt>입력 예시</dt><dd><code>{JSON.stringify(selectedInspectorField.inputExample)}</code></dd>
                    <dt>수식</dt><dd>{selectedInspectorField.isPrimaryKey ? "허용하지 않음" : "타입 검증 후 저장 가능"}</dd>
                  </dl>
                </article>
              {:else}
                <div class="empty-panel">필드 정의를 불러오는 중입니다.</div>
              {/if}
            {:else if inspectorSelection.kind === "compareLimit"}
              <div class="empty-panel">
                100개가 넘는 행이 선택되었습니다. 대화형 비교를 위해 선택 범위를
                100개 이하로 줄여 주세요.
              </div>
            {:else if inspectorSelection.kind === "compare"}
              <div class="comparison-toolbar">
                <strong>{inspectorSelection.rowIds.length}개 행 비교</strong>
                <label><input type="checkbox" bind:checked={differencesOnly} /> 차이만 보기</label>
              </div>
              <div class="comparison-scroll">
                <table class="comparison-table">
                  <thead>
                    <tr>
                      <th>필드</th>
                      {#each rowComparison?.rowIds ?? [] as rowId}
                        <th title={rowId}>{rowId.slice(0, 12)}</th>
                      {/each}
                    </tr>
                  </thead>
                  <tbody>
                    {#each visibleComparisonFields as field}
                      <tr class:different={!field.allEqual}>
                        <th><span>{field.path}</span><small>{field.typeName}</small></th>
                        {#each field.values as cell}
                          <td>
                            <code>{cell.present ? JSON.stringify(cell.value) : "⟨없음⟩"}</code>
                            {#if cell.formula}<small class="formula">{cell.formula}</small>{/if}
                          </td>
                        {/each}
                      </tr>
                    {/each}
                  </tbody>
                </table>
                {#if rowComparison && visibleComparisonFields.length === 0}
                  <div class="empty-panel success">선택한 행의 값과 수식이 같습니다.</div>
                {/if}
              </div>
            {:else}
              {#if inspectorSelection.kind === "create"}
                <div class="create-guide">
                  <strong>새 데이터 추가</strong>
                  <span>검증을 통과하면 JSON 원본의 마지막 행에 추가합니다.</span>
                  {#if rowDraft?.missingRequired.length}
                    <p>필수 입력: {rowDraft.missingRequired.join(", ")}</p>
                  {/if}
                  {#if rowDraft && !rowDraft.editable}
                    <p class="error-text">{rowDraft.readonlyReason}</p>
                  {/if}
                  <details>
                    <summary>입력 가능한 필드 보기</summary>
                    <div class="draft-fields">
                      {#each rowDraft?.fields ?? [] as field}
                        <span>
                          <strong>{field.path}</strong>
                          <code>{field.typeName}</code>
                          {#if field.enumValues.length}<small>{field.enumValues.join(" · ")}</small>{/if}
                          {#if field.foreignKeyTarget}<small>FK → {field.foreignKeyTarget}</small>{/if}
                          {#if draftFieldOptions(field).length}
                            <select
                              aria-label={`${field.path} 선택`}
                              onchange={(event) => {
                                if (event.currentTarget.value) {
                                  setDraftField(
                                    field.path,
                                    JSON.parse(event.currentTarget.value),
                                  );
                                }
                              }}
                            >
                              <option value="" disabled selected>값 선택…</option>
                              {#each draftFieldOptions(field) as option}
                                <option value={JSON.stringify(option)}>{String(option)}</option>
                              {/each}
                            </select>
                          {/if}
                          {#if field.isOptional && field.typeName.includes("string")}
                            <div class="null-actions">
                              <button onclick={() => setDraftField(field.path, "")}>빈 문자열</button>
                              <button onclick={() => setDraftField(field.path, null)}>null</button>
                            </div>
                          {/if}
                        </span>
                      {/each}
                    </div>
                  </details>
                </div>
              {/if}
              <div class="json-editor-wrap contextual">
                {#if inspectorFormulaHints.length > 0}
                  <div class="formula-hint-list">
                    {#each inspectorFormulaHints as hint}
                      <div>
                        <span>{hint.path.join(".")}</span>
                        <code>{inlineJsonValue(hint.value)}</code>
                        <strong>ƒ {hint.formula}</strong>
                      </div>
                    {/each}
                  </div>
                {/if}
                <JsonEditor
                  bind:this={jsonEditor}
                  value={jsonText}
                  formulaHints={inspectorFormulaHints}
                  readonly={inspectorSelection.kind === "create" && rowDraft ? !rowDraft.editable : false}
                  on:change={(event) => (jsonText = event.detail)}
                  on:commit={(event) => {
                    jsonText = event.detail;
                    void applyJson();
                  }}
                  on:select={selectJsonCell}
                />
                <span class="auto-apply-note">
                  {inspectorSelection.kind === "row"
                    ? "값을 벗어나면 자동 검증·적용"
                    : "완성된 행을 벗어나면 자동 검증·추가"}
                </span>
              </div>
            {/if}
          {:else}
            <div class="empty-panel">계산 시트는 JSON source로 내보내지 않습니다.</div>
          {/if}
        {:else if rightTab === "issues"}
          {#if project.diagnostics.length === 0}
            <div class="empty-panel success">검증 오류가 없습니다.</div>
          {:else}
            <div class="issue-list">
              {#each project.diagnostics as issue}
                <article class:error-issue={issue.severity === "error"}>
                  <span>{issue.severity}</span>
                  <strong>{issue.path}</strong>
                  <p>{issue.message}</p>
                  {#if identityRowId(issue)}
                    <div class="identity-actions">
                      <button onclick={() => confirmIdentity(issue, false)}>새 행으로 확인</button>
                      <button onclick={() => confirmIdentity(issue, true)}>기존 ID에 재연결</button>
                    </div>
                  {/if}
                </article>
              {/each}
            </div>
          {/if}
        {:else if rightTab === "diff"}
          {#if diffReport.changes.length === 0}
            <div class="empty-panel">선택한 두 버전 사이에 의미 변경이 없습니다.</div>
          {:else}
            <div class="diff-list">
              {#each diffReport.changes as change}
                <article>
                  <span class="change-kind">{change.kind}</span>
                  <strong>{change.sheetName}{change.rowId ? `[${change.rowId}]` : ""}</strong>
                  <code>{change.path}</code>
                  <div class="before">{JSON.stringify(change.before)}</div>
                  <div class="after">{JSON.stringify(change.after)}</div>
                </article>
              {/each}
            </div>
          {/if}
        {:else if rightTab === "merge"}
          {#if !mergeReport}
            <div class="empty-panel">왼쪽에서 base / ours / theirs를 선택해 병합을 미리 보세요.</div>
          {:else if mergeReport.conflicts.length === 0}
            <div class="empty-panel success">
              필드 단위 자동 병합이 가능합니다.
              {#if mergeReport.recalculation_required}
                적용 후 수식을 재계산하고 검증해야 저장·Stage할 수 있습니다.
              {/if}
              <button class="primary merge-apply" onclick={applyMerge}>Working tree에 적용</button>
            </div>
          {:else}
            <div class="diff-list">
              {#each mergeReport.conflicts as conflict}
                <article>
                  <span class="change-kind">conflict</span>
                  <strong>{conflict.reason}</strong>
                  <code>{conflict.sheet_id}{conflict.row_id ? `[${conflict.row_id}]` : ""} {conflict.path}</code>
                  <div class="before">ours: {JSON.stringify(conflict.ours)}</div>
                  <div class="after">theirs: {JSON.stringify(conflict.theirs)}</div>
                </article>
              {/each}
              <p class="conflict-note">
                v1에서는 같은 필드, delete-vs-edit, 배열 동시 변경, 충돌하는 순서 변경을
                자동 선택하지 않습니다. JSON 또는 시트에서 해결한 뒤 다시 병합하세요.
              </p>
            </div>
          {/if}
        {:else}
          <div class="normalization-list">
            <p>
              최초 저장에서 아래 JSON을 UTF-8/LF·2칸 들여쓰기·스키마 필드 순서로
              정규화합니다. 별도 커밋을 권장합니다.
            </p>
            {#each normalizationPreview as preview}
              <article>
                <strong>{preview.path}</strong>
                <details>
                  <summary>기존 JSON</summary>
                  <pre>{preview.before}</pre>
                </details>
                <details open>
                  <summary>정규화 결과</summary>
                  <pre>{preview.after}</pre>
                </details>
              </article>
            {/each}
          </div>
        {/if}
      </div>
    </aside>

    <footer class="statusbar">
      <span class:error-text={!!error}>{error || status}</span>
      <span>{busy ? "처리 중…" : "Ready"} · v{APP_VERSION}</span>
    </footer>
  </div>
{/if}

{#if project && projectDialogOpen}
  <div class="modal-backdrop" role="presentation">
    <div class="project-dialog" role="dialog" aria-modal="true" aria-labelledby="project-dialog-title">
      <p class="eyebrow">{projectDialogMode === "save" ? "SAVE POLYSHEET" : "CONNECT SCHEMA"}</p>
      <h2 id="project-dialog-title">
        {projectDialogMode === "save" ? "처음 저장하기" : ".poly 정의 연결하기"}
      </h2>
      {#if projectDialogMode === "save"}
        <label>
          <span>PolySheet 프로젝트 폴더</span>
          <div class="path-input">
            <input bind:value={saveAsPath} placeholder="D:\project\game.polysheet" />
            <button onclick={() => pickDirectory("save")}>찾아보기</button>
          </div>
        </label>
      {/if}
      <label>
        <span>.poly 스키마 파일 (선택)</span>
        <div class="path-input">
          <input bind:value={schemaPath} placeholder="나중에 연결할 수 있습니다" />
          <button onclick={pickSchema}>파일 선택</button>
        </div>
      </label>
      {#if schemaPath}
        <label>
          <span>.sources.toml 파일 (선택)</span>
          <div class="path-input">
            <input bind:value={sourcesPath} />
            <button onclick={pickSources}>파일 선택</button>
          </div>
        </label>
        <label>
          <span>데이터 루트 (선택)</span>
          <div class="path-input">
            <input bind:value={dataRoot} />
            <button onclick={() => pickDirectory("data")}>찾아보기</button>
          </div>
        </label>
      {/if}
      {#if error}<p class="error">{error}</p>{/if}
      <div class="dialog-actions">
        <button onclick={() => (projectDialogOpen = false)} disabled={busy}>취소</button>
        <button class="primary" onclick={completeProjectDialog} disabled={busy}>
          {busy ? "처리 중…" : projectDialogMode === "save" ? "저장" : "연결"}
        </button>
      </div>
    </div>
  </div>
{/if}
