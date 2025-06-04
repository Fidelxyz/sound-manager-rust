<template>
  <div :class="cx('root')" v-bind="ptmi('root')">
    <template v-if="loading && loadingMode === 'mask'">
      <div :class="cx('mask')" v-bind="ptm('mask')">
        <slot name="loadingicon" :class="cx('loadingIcon')">
          <i
            v-if="loadingIcon"
            :class="[cx('loadingIcon'), 'pi-spin', loadingIcon]"
            v-bind="ptm('loadingIcon')"
          />
          <SpinnerIcon
            v-else
            spin
            :class="cx('loadingIcon')"
            v-bind="ptm('loadingIcon')"
          />
        </slot>
      </div>
    </template>
    <IconField
      v-if="filter"
      :unstyled="unstyled"
      :pt="{ ...ptm('pcFilter'), ...ptm('pcFilterContainer') }"
      :class="cx('pcFilterContainer')"
    >
      <InputText
        v-model="filterValue"
        autocomplete="off"
        :class="cx('pcFilterInput')"
        :placeholder="filterPlaceholder"
        :unstyled="unstyled"
        @keyup="onFilterKeyup"
        :pt="ptm('pcFilterInput')"
      />
      <InputIcon :unstyled="unstyled" :pt="ptm('pcFilterIconContainer')">
        <!--TODO: searchicon deprecated since v4.0-->
        <slot
          :name="$slots.filtericon ? 'filtericon' : 'searchicon'"
          :class="cx('filterIcon')"
        >
          <SearchIcon :class="cx('filterIcon')" v-bind="ptm('filterIcon')" />
        </slot>
      </InputIcon>
    </IconField>
    <div
      class="overflow-hidden!"
      :class="cx('wrapper')"
      :style="{ maxHeight: scrollHeight }"
      v-bind="ptm('wrapper')"
    >
      <slot
        name="header"
        :value="value"
        :expandedKeys="expandedKeys"
        :selectionKeys="selectionKeys"
      />
      <ul
        :class="cx('rootChildren')"
        role="tree"
        :aria-labelledby="ariaLabelledby"
        :aria-label="ariaLabel"
        v-bind="ptm('rootChildren')"
      >
        <TreeNode
          v-for="(node, index) of valueToRender"
          :key="node.key"
          :node="node"
          :templates="$slots"
          :level="level + 1"
          :index="index"
          :root="this"
          :expandedKeys="d_expandedKeys"
          @node-toggle="onNodeToggle"
          @node-click="onNodeClick"
          :selectionMode="selectionMode"
          :selectionKeys="selectionKeys"
          @checkbox-change="onCheckboxChange"
          :loadingMode="loadingMode"
          :draggableType="draggableType"
          :canDrop="canDrop"
          :lastInGroup="index === valueToRender.length - 1"
          :unstyled="unstyled"
          :pt="pt"
        />
      </ul>
      <slot
        name="footer"
        :value="value"
        :expandedKeys="expandedKeys"
        :selectionKeys="selectionKeys"
      />
    </div>
  </div>
</template>

<script>
import { resolveFieldData } from "@primeuix/utils/object";
import SearchIcon from "@primevue/icons/search";
import SpinnerIcon from "@primevue/icons/spinner";
import IconField from "primevue/iconfield";
import InputIcon from "primevue/inputicon";
import InputText from "primevue/inputtext";
import { defineComponent } from "vue";
import BaseTree from "./BaseTree.vue";
import TreeNode from "./TreeNode.vue";

import { extractInstruction } from "@atlaskit/pragmatic-drag-and-drop-hitbox/tree-item";
import { combine } from "@atlaskit/pragmatic-drag-and-drop/combine";
import { monitorForElements } from "@atlaskit/pragmatic-drag-and-drop/element/adapter";

export default defineComponent({
  name: "Tree",
  extends: BaseTree,
  inheritAttrs: false,
  props: {
    draggableType: {
      type: String,
      default: null,
    },
    canDrop: {
      type: Function,
      default: () => false,
    },
  },
  emits: [
    "node-expand",
    "node-collapse",
    "update:expandedKeys",
    "update:selectionKeys",
    "node-select",
    "node-unselect",
    "filter",
    "node-reorder",
  ],
  data() {
    return {
      d_expandedKeys: this.expandedKeys || {},
      filterValue: null,
      unregisterDraggingMonitor: () => {},
    };
  },
  watch: {
    expandedKeys(newValue) {
      this.d_expandedKeys = newValue;
    },
  },
  mounted() {
    this.registerDraggingMonitor();
  },
  beforeUnmount() {
    this.unregisterDraggingMonitor();
  },
  methods: {
    onNodeToggle(node) {
      const key = node.key;

      if (this.d_expandedKeys[key]) {
        delete this.d_expandedKeys[key];
        this.$emit("node-collapse", node);
      } else {
        this.d_expandedKeys[key] = true;
        this.$emit("node-expand", node);
      }

      this.d_expandedKeys = { ...this.d_expandedKeys };
      this.$emit("update:expandedKeys", this.d_expandedKeys);
    },
    onNodeClick(event) {
      if (this.selectionMode != null && event.node.selectable !== false) {
        const metaSelection = event.nodeTouched ? false : this.metaKeySelection;
        const _selectionKeys = metaSelection
          ? this.handleSelectionWithMetaKey(event)
          : this.handleSelectionWithoutMetaKey(event);

        this.$emit("update:selectionKeys", _selectionKeys);
      }
    },
    onCheckboxChange(event) {
      this.$emit("update:selectionKeys", event.selectionKeys);

      if (event.check) this.$emit("node-select", event.node);
      else this.$emit("node-unselect", event.node);
    },
    handleSelectionWithMetaKey(event) {
      const originalEvent = event.originalEvent;
      const node = event.node;
      const metaKey = originalEvent.metaKey || originalEvent.ctrlKey;
      const selected = this.isNodeSelected(node);
      let _selectionKeys = {};

      if (selected && metaKey) {
        if (this.isSingleSelectionMode()) {
          _selectionKeys = {};
        } else {
          _selectionKeys = { ...this.selectionKeys };
          delete _selectionKeys[node.key];
        }

        this.$emit("node-unselect", node);
      } else {
        if (this.isSingleSelectionMode()) {
          _selectionKeys = {};
        } else if (this.isMultipleSelectionMode()) {
          _selectionKeys = !metaKey
            ? {}
            : this.selectionKeys
              ? { ...this.selectionKeys }
              : {};
        }

        _selectionKeys[node.key] = true;
        this.$emit("node-select", node);
      }

      return _selectionKeys;
    },
    handleSelectionWithoutMetaKey(event) {
      const node = event.node;
      const selected = this.isNodeSelected(node);
      let _selectionKeys;

      if (this.isSingleSelectionMode()) {
        if (selected) {
          _selectionKeys = {};
          this.$emit("node-unselect", node);
        } else {
          _selectionKeys = {};
          _selectionKeys[node.key] = true;
          this.$emit("node-select", node);
        }
      } else {
        if (selected) {
          _selectionKeys = { ...this.selectionKeys };
          delete _selectionKeys[node.key];

          this.$emit("node-unselect", node);
        } else {
          _selectionKeys = this.selectionKeys ? { ...this.selectionKeys } : {};
          _selectionKeys[node.key] = true;

          this.$emit("node-select", node);
        }
      }

      return _selectionKeys;
    },
    isSingleSelectionMode() {
      return this.selectionMode === "single";
    },
    isMultipleSelectionMode() {
      return this.selectionMode === "multiple";
    },
    isNodeSelected(node) {
      return this.selectionMode && this.selectionKeys
        ? this.selectionKeys[node.key] === true
        : false;
    },
    isChecked(node) {
      return this.selectionKeys
        ? this.selectionKeys[node.key] && this.selectionKeys[node.key].checked
        : false;
    },
    isNodeLeaf(node) {
      return node.leaf === false ? false : !node.children?.length;
    },
    onFilterKeyup(event) {
      if (event.code === "Enter" || event.code === "NumpadEnter") {
        event.preventDefault();
      }

      this.$emit("filter", {
        originalEvent: event,
        value: event.target?.value,
      });
    },
    findFilteredNodes(node, paramsWithoutNode) {
      if (node) {
        let matched = false;

        if (node.children) {
          const childNodes = [...node.children];

          node.children = [];

          for (const childNode of childNodes) {
            const copyChildNode = { ...childNode };

            if (this.isFilterMatched(copyChildNode, paramsWithoutNode)) {
              matched = true;
              node.children.push(copyChildNode);
            }
          }
        }

        if (matched) {
          return true;
        }
      }
    },
    isFilterMatched(node, { searchFields, filterText, strict }) {
      let matched = false;

      for (const field of searchFields) {
        const fieldValue = String(
          resolveFieldData(node, field),
        ).toLocaleLowerCase(this.filterLocale);

        if (fieldValue.indexOf(filterText) > -1) {
          matched = true;
        }
      }

      if (!matched || (strict && !this.isNodeLeaf(node))) {
        matched =
          this.findFilteredNodes(node, { searchFields, filterText, strict }) ||
          matched;
      }

      return matched;
    },
    // ========== Drag and Drop BEGIN ==========
    registerDraggingMonitor() {
      if (!this.draggableType) return;

      const tree = this;
      this.unregisterDraggingMonitor = combine(
        // Monitor reordering
        monitorForElements({
          canMonitor: ({ source }) => source.data.type === tree.draggableType,
          onDrop({ location, source }) {
            if (location.current.dropTargets.length === 0) return;

            const sourceData = source.data;
            const targetData = location.current.dropTargets[0].data;
            const instruction = extractInstruction(targetData);

            const sourceKey = sourceData.key;
            const targetKey = targetData.key;

            // take the source node from the tree
            let sourceNode = null;
            if (
              instruction?.type === "reorder-above" ||
              instruction?.type === "reorder-below" ||
              instruction?.type === "make-child"
            ) {
              sourceNode = tree.takeNode(sourceKey);
            }
            if (!sourceNode) return;

            // insert the source node into the target node
            let inserted;
            switch (instruction?.type) {
              case "reorder-above":
                inserted = tree.insertNodeAbove(sourceNode, targetKey);
                break;
              case "reorder-below":
                inserted = tree.insertNodeBelow(sourceNode, targetKey);
                break;
              case "make-child":
                inserted = tree.appendChildNode(sourceNode, targetKey);
                break;
              default:
                return;
            }
            if (!inserted) return;

            tree.$emit("node-reorder", {
              sourceKey: sourceKey,
              targetParentKey: inserted.parentKey,
              targetLocation: inserted.location,
            });
          },
        }),
      );
    },
    takeNode(key) {
      const takeNode = (nodes) => {
        for (let i = 0; i < nodes.length; i++) {
          const node = nodes[i];
          if (node.key === key) {
            return nodes.splice(i, 1)[0];
          }

          if (node.children) {
            const taked = takeNode(node.children);
            if (taked) return taked;
          }
        }

        return null;
      };
      return takeNode(this.value);
    },
    insertNodeAbove(node, targetKey) {
      const insertNodeAbove = (nodes, parent) => {
        for (let i = 0; i < nodes.length; i++) {
          const current = nodes[i];
          if (current.key === targetKey) {
            nodes.splice(i, 0, node);
            return { parentKey: parent ? parent.key : null, location: i };
          }

          if (current.children) {
            const inserted = insertNodeAbove(current.children, current);
            if (inserted) return inserted;
          }
        }

        return null;
      };
      return insertNodeAbove(this.value);
    },
    insertNodeBelow(node, targetKey) {
      const insertNodeBelow = (nodes, parent) => {
        for (let i = 0; i < nodes.length; i++) {
          const current = nodes[i];
          if (current.key === targetKey) {
            nodes.splice(i + 1, 0, node);
            return { parentKey: parent ? parent.key : null, location: i + 1 };
          }

          if (current.children) {
            const inserted = insertNodeBelow(current.children, current);
            if (inserted) return inserted;
          }
        }

        return null;
      };
      return insertNodeBelow(this.value);
    },
    appendChildNode(node, targetKey) {
      const appendChildNode = (nodes) => {
        for (let i = 0; i < nodes.length; i++) {
          const current = nodes[i];
          if (current.key === targetKey) {
            if (!current.children) current.children = [];
            current.children.push(node);
            return {
              parentKey: current.key,
              location: current.children.length - 1,
            };
          }

          if (current.children) {
            const inserted = appendChildNode(current.children);
            if (inserted) return inserted;
          }
        }

        return null;
      };
      return appendChildNode(this.value);
    },
    // ========== Drag and Drop END ==========
  },
  computed: {
    filteredValue() {
      if (!this.filterValue) return;

      const filteredNodes = [];
      const searchFields =
        this.filterBy instanceof Function
          ? [this.filterBy]
          : this.filterBy.split(",");
      const filterText = this.filterValue
        .trim()
        .toLocaleLowerCase(this.filterLocale);
      const strict = this.filterMode === "strict";

      for (const node of this.value) {
        const _node = { ...node };
        const paramsWithoutNode = { searchFields, filterText, strict };

        if (
          (strict &&
            (this.findFilteredNodes(_node, paramsWithoutNode) ||
              this.isFilterMatched(_node, paramsWithoutNode))) ||
          (!strict &&
            (this.isFilterMatched(_node, paramsWithoutNode) ||
              this.findFilteredNodes(_node, paramsWithoutNode)))
        ) {
          filteredNodes.push(_node);
        }
      }

      return filteredNodes;
    },
    valueToRender() {
      if (this.filterValue && this.filterValue.trim().length > 0)
        return this.filteredValue;
      return this.value;
    },
  },
  components: {
    TreeNode,
    InputText,
    InputIcon,
    IconField,
    SearchIcon,
    SpinnerIcon,
  },
});
</script>
