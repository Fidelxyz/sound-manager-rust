<template>
  <template v-if="!empty">
    <tr
      v-if="
        templates['groupheader'] &&
        rowGroupMode === 'subheader' &&
        shouldRenderRowGroupHeader
      "
      :class="cx('rowGroupHeader')"
      :style="rowGroupHeaderStyle"
      role="row"
      v-bind="ptm('rowGroupHeader')"
    >
      <td
        :colspan="columnsLength - 1"
        v-bind="{ ...getColumnPT('bodycell'), ...ptm('rowGroupHeaderCell') }"
      >
        <button
          v-if="expandableRowGroups"
          :class="cx('rowToggleButton')"
          @click="onRowGroupToggle"
          type="button"
          v-bind="ptm('rowToggleButton')"
        >
          <component
            v-if="
              templates['rowtoggleicon'] || templates['rowgrouptogglericon']
            "
            :is="templates['rowtoggleicon'] || templates['rowgrouptogglericon']"
            :expanded="isRowGroupExpanded"
          />
          <template v-else>
            <span
              v-if="isRowGroupExpanded && expandedRowIcon"
              :class="[cx('rowToggleIcon'), expandedRowIcon]"
              v-bind="ptm('rowToggleIcon')"
            />
            <ChevronDownIcon
              v-else-if="isRowGroupExpanded && !expandedRowIcon"
              :class="cx('rowToggleIcon')"
              v-bind="ptm('rowToggleIcon')"
            />
            <span
              v-else-if="!isRowGroupExpanded && collapsedRowIcon"
              :class="[cx('rowToggleIcon'), collapsedRowIcon]"
              v-bind="ptm('rowToggleIcon')"
            />
            <ChevronRightIcon
              v-else-if="!isRowGroupExpanded && !collapsedRowIcon"
              :class="cx('rowToggleIcon')"
              v-bind="ptm('rowToggleIcon')"
            />
          </template>
        </button>
        <component
          :is="templates['groupheader']"
          :data="rowData"
          :index="rowIndex"
        />
      </td>
    </tr>
    <tr
      v-if="expandableRowGroups ? isRowGroupExpanded : true"
      ref="row"
      :class="rowClasses"
      :style="rowStyles"
      :tabindex="rowTabindex"
      role="row"
      :aria-selected="selectionMode ? isSelected : null"
      @click="onRowClick"
      @dblclick="onRowDblClick"
      @contextmenu="onRowRightClick"
      @touchend="onRowTouchEnd"
      @keydown.self="onRowKeyDown"
      @mousedown="onRowMouseDown"
      @dragstart="onRowDragStart"
      @dragover="onRowDragOver"
      @dragleave="onRowDragLeave"
      @dragend="onRowDragEnd"
      @drop="onRowDrop"
      @vue:mounted="registerDraggable"
      @vue:unmounted="unregisterDraggable"
      v-bind="getBodyRowPTOptions('bodyRow')"
      :data-p-index="rowIndex"
      :data-p-selectable-row="selectionMode ? true : false"
      :data-p-selected="selection && isSelected"
      :data-p-selected-contextmenu="
        contextMenuSelection && isSelectedWithContextMenu
      "
    >
      <template v-for="(col, i) of columns">
        <DTBodyCell
          v-if="shouldRenderBodyCell(col)"
          :key="columnProp(col, 'columnKey') || columnProp(col, 'field') || i"
          :rowData="rowData"
          :column="col"
          :rowIndex="rowIndex"
          :index="i"
          :selected="isSelected"
          :frozenRow="frozenRow"
          :rowspan="
            rowGroupMode === 'rowspan' ? calculateRowGroupSize(col) : null
          "
          :editMode="editMode"
          :editing="editMode === 'row' && isRowEditing"
          :editingMeta="editingMeta"
          :virtualScrollerContentProps="virtualScrollerContentProps"
          :ariaControls="expandedRowId + '_' + rowIndex + '_expansion'"
          :name="nameAttributeSelector"
          :isRowExpanded="d_rowExpanded"
          :expandedRowIcon="expandedRowIcon"
          :collapsedRowIcon="collapsedRowIcon"
          :editButtonProps="editButtonProps"
          @radio-change="onRadioChange"
          @checkbox-change="onCheckboxChange"
          @row-toggle="onRowToggle"
          @cell-edit-init="onCellEditInit"
          @cell-edit-complete="onCellEditComplete"
          @cell-edit-cancel="onCellEditCancel"
          @row-edit-init="onRowEditInit"
          @row-edit-save="onRowEditSave"
          @row-edit-cancel="onRowEditCancel"
          @editing-meta-change="onEditingMetaChange"
          :unstyled="unstyled"
          :pt="pt"
        />
      </template>
    </tr>
    <tr
      v-if="templates['expansion'] && expandedRows && d_rowExpanded"
      :id="expandedRowId + '_' + rowIndex + '_expansion'"
      :class="cx('rowExpansion')"
      role="row"
      v-bind="ptm('rowExpansion')"
    >
      <td
        :colspan="columnsLength"
        v-bind="{ ...getColumnPT('bodycell'), ...ptm('rowExpansionCell') }"
      >
        <component
          :is="templates['expansion']"
          :data="rowData"
          :index="rowIndex"
        />
      </td>
    </tr>
    <tr
      v-if="
        templates['groupfooter'] &&
        rowGroupMode === 'subheader' &&
        shouldRenderRowGroupFooter
      "
      :class="cx('rowGroupFooter')"
      role="row"
      v-bind="ptm('rowGroupFooter')"
    >
      <td
        :colspan="columnsLength - 1"
        v-bind="{ ...getColumnPT('bodycell'), ...ptm('rowGroupFooterCell') }"
      >
        <component
          :is="templates['groupfooter']"
          :data="rowData"
          :index="rowIndex"
        />
      </td>
    </tr>
  </template>
  <tr
    v-else
    :class="cx('emptyMessage')"
    role="row"
    v-bind="ptm('emptyMessage')"
  >
    <td
      :colspan="columnsLength"
      v-bind="{ ...getColumnPT('bodycell'), ...ptm('emptyMessageCell') }"
    >
      <component v-if="templates.empty" :is="templates.empty" />
    </td>
  </tr>
</template>

<script>
import { equals, isNotEmpty, resolveFieldData } from "@primeuix/utils/object";
import BaseComponent from "@primevue/core/basecomponent";
import { getVNodeProp } from "@primevue/core/utils";
import ChevronDownIcon from "@primevue/icons/chevrondown";
import ChevronRightIcon from "@primevue/icons/chevronright";
import { defineComponent, mergeProps } from "vue";
import BodyCell from "./BodyCell.vue";

import { draggable } from "@atlaskit/pragmatic-drag-and-drop/element/adapter";
import { setCustomNativeDragPreview } from "@atlaskit/pragmatic-drag-and-drop/element/set-custom-native-drag-preview";

export default defineComponent({
  name: "BodyRow",
  hostName: "DataTable",
  extends: BaseComponent,
  emits: [
    "rowgroup-toggle",
    "row-click",
    "row-dblclick",
    "row-rightclick",
    "row-touchend",
    "row-keydown",
    "row-mousedown",
    "row-dragstart",
    "row-dragover",
    "row-dragleave",
    "row-dragend",
    "row-drop",
    "row-toggle",
    "radio-change",
    "checkbox-change",
    "cell-edit-init",
    "cell-edit-complete",
    "cell-edit-cancel",
    "row-edit-init",
    "row-edit-save",
    "row-edit-cancel",
    "editing-meta-change",
  ],
  props: {
    rowData: {
      type: Object,
      default: null,
    },
    index: {
      type: Number,
      default: 0,
    },
    value: {
      type: Array,
      default: null,
    },
    columns: {
      type: null,
      default: null,
    },
    frozenRow: {
      type: Boolean,
      default: false,
    },
    empty: {
      type: Boolean,
      default: false,
    },
    rowGroupMode: {
      type: String,
      default: null,
    },
    groupRowsBy: {
      type: [Array, String, Function],
      default: null,
    },
    expandableRowGroups: {
      type: Boolean,
      default: false,
    },
    expandedRowGroups: {
      type: Array,
      default: null,
    },
    first: {
      type: Number,
      default: 0,
    },
    dataKey: {
      type: [String, Function],
      default: null,
    },
    dragPreviewKey: {
      type: [String, Function],
      default: null,
    },
    draggableType: {
      type: String,
      default: null,
    },
    expandedRowIcon: {
      type: String,
      default: null,
    },
    collapsedRowIcon: {
      type: String,
      default: null,
    },
    expandedRows: {
      type: [Array, Object],
      default: null,
    },
    selection: {
      type: [Array, Object],
      default: null,
    },
    selectionKeys: {
      type: null,
      default: null,
    },
    selectionMode: {
      type: String,
      default: null,
    },
    contextMenu: {
      type: Boolean,
      default: false,
    },
    contextMenuSelection: {
      type: Object,
      default: null,
    },
    rowClass: {
      type: null,
      default: null,
    },
    rowStyle: {
      type: null,
      default: null,
    },
    rowGroupHeaderStyle: {
      type: null,
      default: null,
    },
    editMode: {
      type: String,
      default: null,
    },
    compareSelectionBy: {
      type: String,
      default: "deepEquals",
    },
    editingRows: {
      type: Array,
      default: null,
    },
    editingRowKeys: {
      type: null,
      default: null,
    },
    editingMeta: {
      type: Object,
      default: null,
    },
    templates: {
      type: null,
      default: null,
    },
    scrollable: {
      type: Boolean,
      default: false,
    },
    editButtonProps: {
      type: Object,
      default: null,
    },
    virtualScrollerContentProps: {
      type: Object,
      default: null,
    },
    isVirtualScrollerDisabled: {
      type: Boolean,
      default: false,
    },
    expandedRowId: {
      type: String,
      default: null,
    },
    nameAttributeSelector: {
      type: String,
      default: null,
    },
  },
  data() {
    return {
      d_rowExpanded: false,
      unregisterDraggable: () => {},
    };
  },
  watch: {
    expandedRows: {
      deep: true,
      immediate: true,
      handler(newValue) {
        this.d_rowExpanded = this.dataKey
          ? newValue?.[resolveFieldData(this.rowData, this.dataKey)] !==
            undefined
          : newValue?.some((d) => this.equals(this.rowData, d));
      },
    },
  },
  methods: {
    columnProp(col, prop) {
      return getVNodeProp(col, prop);
    },
    //@todo - update this method
    getColumnPT(key) {
      const columnMetaData = {
        parent: {
          instance: this,
          props: this.$props,
          state: this.$data,
        },
      };

      return mergeProps(
        this.ptm(`column.${key}`, { column: columnMetaData }),
        this.ptm(`column.${key}`, columnMetaData),
        this.ptmo(this.columnProp({}, "pt"), key, columnMetaData),
      );
    },
    //@todo - update this method
    getBodyRowPTOptions(key) {
      const datatable = this.$parentInstance?.$parentInstance;

      return this.ptm(key, {
        context: {
          index: this.rowIndex,
          selectable: datatable?.rowHover || datatable?.selectionMode,
          selected: this.isSelected,
          stripedRows: datatable?.stripedRows || false,
        },
      });
    },
    shouldRenderBodyCell(column) {
      const isHidden = this.columnProp(column, "hidden");

      if (!this.rowGroupMode || isHidden) {
        return !isHidden;
      }

      const field = this.columnProp(column, "field");

      if (this.rowGroupMode === "subheader") {
        return this.groupRowsBy !== field;
      }
      if (this.rowGroupMode === "rowspan") {
        if (!this.isGrouped(column)) return true;

        const prevRowData = this.value[this.rowIndex - 1];
        if (!prevRowData) return true;

        const currentRowFieldData = resolveFieldData(
          this.value[this.rowIndex],
          field,
        );
        const previousRowFieldData = resolveFieldData(prevRowData, field);
        return currentRowFieldData !== previousRowFieldData;
      }
    },
    calculateRowGroupSize(column) {
      if (!this.isGrouped(column)) return null;

      let index = this.rowIndex;
      const field = this.columnProp(column, "field");
      const currentRowFieldData = resolveFieldData(this.value[index], field);
      let nextRowFieldData = currentRowFieldData;
      let groupRowSpan = 0;

      if (this.d_rowExpanded) groupRowSpan++;

      while (currentRowFieldData === nextRowFieldData) {
        groupRowSpan++;
        const nextRowData = this.value[++index];

        if (nextRowData) {
          nextRowFieldData = resolveFieldData(nextRowData, field);
        } else {
          break;
        }
      }

      return groupRowSpan === 1 ? null : groupRowSpan;
    },
    isGrouped(column) {
      const field = this.columnProp(column, "field");

      if (!this.groupRowsBy || !field) return false;

      return Array.isArray(this.groupRowsBy)
        ? this.groupRowsBy.indexOf(field) > -1
        : this.groupRowsBy === field;
    },
    findIndexInSelection(data) {
      return this.findIndex(data, this.selection);
    },
    findIndex(data, collection) {
      let index = -1;

      if (collection?.length) {
        for (let i = 0; i < collection.length; i++) {
          if (this.equals(data, collection[i])) {
            index = i;
            break;
          }
        }
      }

      return index;
    },
    equals(data1, data2) {
      return this.compareSelectionBy === "equals"
        ? data1 === data2
        : equals(data1, data2, this.dataKey);
    },
    onRowGroupToggle(event) {
      this.$emit("rowgroup-toggle", {
        originalEvent: event,
        data: this.rowData,
      });
    },
    onRowClick(event) {
      this.$emit("row-click", {
        originalEvent: event,
        data: this.rowData,
        index: this.rowIndex,
      });
    },
    onRowDblClick(event) {
      this.$emit("row-dblclick", {
        originalEvent: event,
        data: this.rowData,
        index: this.rowIndex,
      });
    },
    onRowRightClick(event) {
      this.$emit("row-rightclick", {
        originalEvent: event,
        data: this.rowData,
        index: this.rowIndex,
      });
    },
    onRowTouchEnd(event) {
      this.$emit("row-touchend", event);
    },
    onRowKeyDown(event) {
      this.$emit("row-keydown", {
        originalEvent: event,
        data: this.rowData,
        index: this.rowIndex,
      });
    },
    onRowMouseDown(event) {
      this.$emit("row-mousedown", event);
    },
    onRowDragStart(event) {
      this.$emit("row-dragstart", {
        originalEvent: event,
        index: this.rowIndex,
      });
    },
    onRowDragOver(event) {
      this.$emit("row-dragover", {
        originalEvent: event,
        index: this.rowIndex,
      });
    },
    onRowDragLeave(event) {
      this.$emit("row-dragleave", event);
    },
    onRowDragEnd(event) {
      this.$emit("row-dragend", event);
    },
    onRowDrop(event) {
      this.$emit("row-drop", event);
    },
    onRowToggle(event) {
      this.d_rowExpanded = !this.d_rowExpanded;

      this.$emit("row-toggle", { ...event, expanded: this.d_rowExpanded });
    },
    onRadioChange(event) {
      this.$emit("radio-change", event);
    },
    onCheckboxChange(event) {
      this.$emit("checkbox-change", event);
    },
    onCellEditInit(event) {
      this.$emit("cell-edit-init", event);
    },
    onCellEditComplete(event) {
      this.$emit("cell-edit-complete", event);
    },
    onCellEditCancel(event) {
      this.$emit("cell-edit-cancel", event);
    },
    onRowEditInit(event) {
      this.$emit("row-edit-init", event);
    },
    onRowEditSave(event) {
      this.$emit("row-edit-save", event);
    },
    onRowEditCancel(event) {
      this.$emit("row-edit-cancel", event);
    },
    onEditingMetaChange(event) {
      this.$emit("editing-meta-change", event);
    },
    getVirtualScrollerProp(option, options) {
      options = options || this.virtualScrollerContentProps;

      return options ? options[option] : null;
    },
    // ========== Drag and Drop BEGIN ==========
    registerDraggable() {
      if (!this.draggableType) return;

      const draggableArgs = {
        element: this.$refs.row,
        getInitialData: () => ({
          type: this.draggableType,
          key: this.dataKey
            ? resolveFieldData(this.rowData, this.dataKey)
            : this.rowIndex,
          data: this.rowData,
        }),
      };
      if (this.dragPreviewKey) {
        draggableArgs.onGenerateDragPreview = ({ nativeSetDragImage }) => {
          setCustomNativeDragPreview({
            getOffset: ({ container }) => {
              const offset = {
                x: container.offsetWidth / 2,
                y: container.offsetHeight / 2,
              };
              return offset;
            },
            render: ({ container }) => {
              const preview = document.createElement("div");
              preview.textContent = resolveFieldData(
                this.rowData,
                this.dragPreviewKey,
              );
              container.appendChild(preview);
            },
            nativeSetDragImage,
          });
        };
      }
      this.unregisterDraggable = draggable(draggableArgs);
    },
    // ========== Drag and Drop END ==========
  },
  computed: {
    rowIndex() {
      const getItemOptions = this.getVirtualScrollerProp("getItemOptions");

      return getItemOptions ? getItemOptions(this.index).index : this.index;
    },
    rowStyles() {
      return this.rowStyle?.(this.rowData);
    },
    rowClasses() {
      const rowStyleClass = [];
      let columnSelectionMode = null;

      if (this.rowClass) {
        const rowClassValue = this.rowClass(this.rowData);

        if (rowClassValue) {
          rowStyleClass.push(rowClassValue);
        }
      }

      if (this.columns) {
        for (const col of this.columns) {
          const _selectionMode = this.columnProp(col, "selectionMode");

          if (isNotEmpty(_selectionMode)) {
            columnSelectionMode = _selectionMode;
            break;
          }
        }
      }

      return [
        this.cx("row", {
          rowData: this.rowData,
          index: this.rowIndex,
          columnSelectionMode,
        }),
        rowStyleClass,
      ];
    },
    rowTabindex() {
      if (
        this.selection === null &&
        (this.selectionMode === "single" || this.selectionMode === "multiple")
      ) {
        return this.rowIndex === 0 ? 0 : -1;
      }

      return -1;
    },
    isRowEditing() {
      if (!this.rowData || !this.editingRows) return false;
      if (!this.dataKey) {
        return this.findIndex(this.rowData, this.editingRows) > -1;
      }
      if (!this.editingRowKeys) return false;

      return (
        this.editingRowKeys[resolveFieldData(this.rowData, this.dataKey)] !==
        undefined
      );
    },
    isRowGroupExpanded() {
      if (!this.expandableRowGroups || !this.expandedRowGroups) return false;

      const groupFieldValue = resolveFieldData(this.rowData, this.groupRowsBy);
      return this.expandedRowGroups.indexOf(groupFieldValue) > -1;
    },
    isSelected() {
      if (!this.rowData || !this.selection) return false;

      if (this.dataKey) {
        if (!this.selectionKeys) return false;
        return (
          this.selectionKeys[resolveFieldData(this.rowData, this.dataKey)] !==
          undefined
        );
      }

      return Array.isArray(this.selection)
        ? this.findIndexInSelection(this.rowData) > -1
        : this.equals(this.rowData, this.selection);
    },
    isSelectedWithContextMenu() {
      if (!this.rowData || !this.contextMenuSelection) return false;

      return this.equals(this.rowData, this.contextMenuSelection);
    },
    shouldRenderRowGroupHeader() {
      const currentRowFieldData = resolveFieldData(
        this.rowData,
        this.groupRowsBy,
      );

      const prevRowData = this.value[this.rowIndex - 1];
      if (!prevRowData) return true;

      const previousRowFieldData = resolveFieldData(
        prevRowData,
        this.groupRowsBy,
      );

      return currentRowFieldData !== previousRowFieldData;
    },
    shouldRenderRowGroupFooter() {
      if (this.expandableRowGroups && !this.isRowGroupExpanded) return false;

      const currentRowFieldData = resolveFieldData(
        this.rowData,
        this.groupRowsBy,
      );

      const nextRowData = this.value[this.rowIndex + 1];
      if (!nextRowData) return true;

      const nextRowFieldData = resolveFieldData(nextRowData, this.groupRowsBy);

      return currentRowFieldData !== nextRowFieldData;
    },
    columnsLength() {
      if (!this.columns) return 0;

      let hiddenColLength = 0;
      for (const column of this.columns) {
        if (this.columnProp(column, "hidden")) hiddenColLength++;
      }
      return this.columns.length - hiddenColLength;
    },
  },
  components: {
    DTBodyCell: BodyCell,
    ChevronDownIcon: ChevronDownIcon,
    ChevronRightIcon: ChevronRightIcon,
  },
});
</script>
