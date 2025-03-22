<template>
  <li
    ref="currentNode"
    :class="[cx('node'), { dragging: dragging }]"
    role="treeitem"
    :aria-label="label(node)"
    :aria-selected="ariaSelected"
    :aria-expanded="expanded"
    :aria-setsize="node.children ? node.children.length : 0"
    :aria-posinset="index + 1"
    :aria-level="level"
    :aria-checked="ariaChecked"
    :tabindex="index === 0 ? 0 : -1"
    @keydown="onKeyDown"
    v-bind="level === 1 ? getPTOptions('node') : ptm('nodeChildren')"
  >
    <div
      class="relative"
      :class="cx('nodeContent')"
      @click="onClick"
      @touchend="onTouchEnd"
      :style="node.style"
      v-bind="getPTOptions('nodeContent')"
      :data-p-selected="checkboxMode ? checked : selected"
      :data-p-selectable="selectable"
    >
      <button
        v-ripple
        type="button"
        :class="cx('nodeToggleButton')"
        @click="toggle"
        tabindex="-1"
        v-bind="getPTOptions('nodeToggleButton')"
      >
        <template v-if="node.loading && loadingMode === 'icon'">
          <!-- TODO: nodetogglericon deprecated since v4.0-->
          <component
            v-if="templates['nodetoggleicon'] || templates['nodetogglericon']"
            :is="templates['nodetoggleicon'] || templates['nodetogglericon']"
            :node="node"
            :expanded="expanded"
            :class="cx('nodeToggleIcon')"
          />
          <SpinnerIcon
            v-else
            spin
            :class="cx('nodeToggleIcon')"
            v-bind="getPTOptions('nodeToggleIcon')"
          />
        </template>
        <template v-else>
          <!-- TODO: togglericon deprecated since v4.0-->
          <component
            v-if="templates['nodetoggleicon'] || templates['togglericon']"
            :is="templates['nodetoggleicon'] || templates['togglericon']"
            :node="node"
            :expanded="expanded"
            :class="cx('nodeToggleIcon')"
          />
          <component
            v-else-if="expanded"
            :is="node.expandedIcon ? 'span' : 'ChevronDownIcon'"
            :class="cx('nodeToggleIcon')"
            v-bind="getPTOptions('nodeToggleIcon')"
          />
          <component
            v-else
            :is="node.collapsedIcon ? 'span' : 'ChevronRightIcon'"
            :class="cx('nodeToggleIcon')"
            v-bind="getPTOptions('nodeToggleIcon')"
          />
        </template>
      </button>
      <Checkbox
        v-if="checkboxMode"
        :defaultValue="checked"
        :binary="true"
        :indeterminate="partialChecked"
        :class="cx('nodeCheckbox')"
        :tabindex="-1"
        :unstyled="unstyled"
        :pt="getPTOptions('pcNodeCheckbox')"
        :data-p-partialchecked="partialChecked"
      >
        <template #icon="slotProps">
          <component
            v-if="templates['checkboxicon']"
            :is="templates['checkboxicon']"
            :checked="slotProps.checked"
            :partialChecked="partialChecked"
            :class="slotProps.class"
          />
        </template>
      </Checkbox>
      <component
        v-if="templates['nodeicon']"
        :is="templates['nodeicon']"
        :node="node"
        :class="[cx('nodeIcon')]"
        v-bind="getPTOptions('nodeIcon')"
      ></component>
      <span
        v-else
        :class="[cx('nodeIcon'), node.icon]"
        v-bind="getPTOptions('nodeIcon')"
      ></span>
      <span
        :class="cx('nodeLabel')"
        v-bind="getPTOptions('nodeLabel')"
        @keydown.stop
      >
        <component
          v-if="templates[node.type] || templates['default']"
          :is="templates[node.type] || templates['default']"
          :node="node"
          :expanded="expanded"
          :selected="checkboxMode ? checked : selected"
        />
        <template v-else>{{ label(node) }}</template>
      </span>
      <DropIndicator
        v-if="dropTargetInstruction"
        :instruction="dropTargetInstruction"
      />
    </div>
    <ul
      v-if="hasChildren && expanded"
      :class="cx('nodeChildren')"
      role="group"
      v-bind="ptm('nodeChildren')"
    >
      <TreeNode
        v-for="childNode of node.children"
        :key="childNode.key"
        :node="childNode"
        :templates="templates"
        :level="level + 1"
        :loadingMode="loadingMode"
        :expandedKeys="expandedKeys"
        @node-toggle="onChildNodeToggle"
        @node-click="onChildNodeClick"
        :selectionMode="selectionMode"
        :selectionKeys="selectionKeys"
        @checkbox-change="propagateUp"
        :unstyled="unstyled"
        :pt="pt"
      />
    </ul>
  </li>
</template>

<script lang="ts">
import { defineComponent } from "vue";
import { find, findSingle, getAttribute } from "@primeuix/utils/dom";
import BaseComponent from "@primevue/core/basecomponent";
import CheckIcon from "@primevue/icons/check";
import ChevronDownIcon from "@primevue/icons/chevrondown";
import ChevronRightIcon from "@primevue/icons/chevronright";
import MinusIcon from "@primevue/icons/minus";
import SpinnerIcon from "@primevue/icons/spinner";
import Checkbox from "primevue/checkbox";
import Ripple from "primevue/ripple";
import type { TreeNode } from "primevue/treenode";
import type { TreeSelectionKeys } from "./Tree";

import {
  draggable,
  dropTargetForElements,
} from "@atlaskit/pragmatic-drag-and-drop/element/adapter";
import {
  attachInstruction,
  extractInstruction,
  type Instruction,
} from "@atlaskit/pragmatic-drag-and-drop-hitbox/tree-item";
import type { CleanupFn } from "@atlaskit/pragmatic-drag-and-drop/dist/types/internal-types";
import DropIndicator from "./dropindicator/DropIndicator.vue";

export default defineComponent({
  name: "TreeNode",
  hostName: "Tree",
  extends: BaseComponent,
  emits: ["node-toggle", "node-click", "checkbox-change"],
  props: {
    node: {
      type: null,
      default: null,
    },
    expandedKeys: {
      type: null,
      default: null,
    },
    loadingMode: {
      type: String,
      default: "mask",
    },
    selectionKeys: {
      type: null,
      default: null,
    },
    selectionMode: {
      type: String,
      default: null,
    },
    templates: {
      type: null,
      default: null,
    },
    level: {
      type: Number,
      default: null,
    },
    index: null,
  },
  data(): {
    nodeTouched: boolean;
    toggleClicked: boolean;
    dragging: boolean;
    dropTargetInstruction: Instruction | null;
    unregisterDraggable: CleanupFn;
    unregisterDropTarget: CleanupFn;
  } {
    return {
      nodeTouched: false,
      toggleClicked: false,
      dragging: false,
      dropTargetInstruction: null,
      unregisterDraggable: () => {},
      unregisterDropTarget: () => {},
    };
  },
  mounted() {
    this.setAllNodesTabIndexes();
    this.registerDraggable();
  },
  beforeUnmount() {
    this.unregisterDraggable();
    this.unregisterDropTarget();
  },
  methods: {
    toggle() {
      this.$emit("node-toggle", this.node);
      this.toggleClicked = true;
    },
    label(node: TreeNode): string | undefined {
      return typeof node.label === "function" ? node.label() : node.label;
    },
    onChildNodeToggle(node: TreeNode) {
      this.$emit("node-toggle", node);
    },
    getPTOptions(key) {
      return this.ptm(key, {
        context: {
          node: this.node,
          index: this.index,
          expanded: this.expanded,
          selected: this.selected,
          checked: this.checked,
          partialChecked: this.partialChecked,
          leaf: this.leaf,
        },
      });
    },
    onClick(event: UIEvent) {
      if (
        this.toggleClicked ||
        getAttribute(event.target, '[data-pc-section="nodetogglebutton"]') ||
        getAttribute(
          (event.target as Element).parentElement,
          '[data-pc-section="nodetogglebutton"]'
        )
      ) {
        this.toggleClicked = false;

        return;
      }

      if (this.isCheckboxSelectionMode()) {
        if (this.node.selectable !== false) {
          this.toggleCheckbox();
        }
      } else {
        this.$emit("node-click", {
          originalEvent: event,
          nodeTouched: this.nodeTouched,
          node: this.node,
        });
      }

      this.nodeTouched = false;
    },
    onChildNodeClick(event: UIEvent) {
      this.$emit("node-click", event);
    },
    onTouchEnd() {
      this.nodeTouched = true;
    },
    onKeyDown(event: KeyboardEvent) {
      if (!this.isSameNode(event)) return;

      switch (event.code) {
        case "Tab":
          this.onTabKey(event);
          break;
        case "ArrowDown":
          this.onArrowDown(event);
          break;
        case "ArrowUp":
          this.onArrowUp(event);
          break;
        case "ArrowRight":
          this.onArrowRight(event);
          break;
        case "ArrowLeft":
          this.onArrowLeft(event);
          break;
        case "Enter":
        case "NumpadEnter":
        case "Space":
          this.onEnterKey(event);
          break;
        default:
          break;
      }
    },
    onArrowDown(event: KeyboardEvent) {
      const nodeElement =
        (event.target as Element).getAttribute("data-pc-section") ===
        "nodetogglebutton"
          ? (event.target as Element).closest('[role="treeitem"]')
          : (event.target as Element);
      const listElement = nodeElement?.children[1];

      if (listElement) {
        this.focusRowChange(nodeElement, listElement.children[0]);
      } else {
        if (nodeElement?.nextElementSibling) {
          this.focusRowChange(nodeElement, nodeElement.nextElementSibling);
        } else {
          const nextSiblingAncestor =
            this.findNextSiblingOfAncestor(nodeElement);

          if (nextSiblingAncestor) {
            this.focusRowChange(nodeElement, nextSiblingAncestor);
          }
        }
      }

      event.preventDefault();
    },
    onArrowUp(event: KeyboardEvent) {
      const nodeElement = event.target as Element;

      if (nodeElement.previousElementSibling) {
        this.focusRowChange(
          nodeElement,
          nodeElement.previousElementSibling,
          this.findLastVisibleDescendant(nodeElement.previousElementSibling)
        );
      } else {
        const parentNodeElement = this.getParentNodeElement(nodeElement);

        if (parentNodeElement) {
          this.focusRowChange(nodeElement, parentNodeElement);
        }
      }

      event.preventDefault();
    },
    onArrowRight(event: KeyboardEvent) {
      if (this.leaf || this.expanded) return;

      (event.currentTarget as HTMLElement).tabIndex = -1;

      this.$emit("node-toggle", this.node);
      this.$nextTick(() => {
        this.onArrowDown(event);
      });
    },
    onArrowLeft(event: KeyboardEvent) {
      const togglerElement = findSingle(
        event.currentTarget as Element,
        '[data-pc-section="nodetogglebutton"]'
      );

      if (this.level === 0 && !this.expanded) {
        return false;
      }

      if (this.expanded && !this.leaf) {
        (togglerElement as HTMLElement).click();

        return false;
      }

      const target = this.findBeforeClickableNode(
        event.currentTarget as Element
      );

      if (target) {
        this.focusRowChange(event.currentTarget, target);
      }
    },
    onEnterKey(event: KeyboardEvent) {
      this.setTabIndexForSelectionMode(event, this.nodeTouched);
      this.onClick(event);

      event.preventDefault();
    },
    onTabKey() {
      this.setAllNodesTabIndexes();
    },
    setAllNodesTabIndexes() {
      const nodes = find(
        this.$refs.currentNode.closest('[data-pc-section="rootchildren"]'),
        '[role="treeitem"]'
      );

      const hasSelectedNode = [...nodes].some(
        (node) =>
          node.getAttribute("aria-selected") === "true" ||
          node.getAttribute("aria-checked") === "true"
      );

      for (const node of [...nodes]) {
        (node as HTMLElement).tabIndex = -1;
      }

      if (hasSelectedNode) {
        const selectedNodes = [...nodes].filter(
          (node) =>
            node.getAttribute("aria-selected") === "true" ||
            node.getAttribute("aria-checked") === "true"
        );

        (selectedNodes[0] as HTMLElement).tabIndex = 0;

        return;
      }

      ([...nodes][0] as HTMLElement).tabIndex = 0;
    },
    setTabIndexForSelectionMode(event: UIEvent, nodeTouched: boolean) {
      if (this.selectionMode !== null) {
        const elements = [
          ...find(this.$refs.currentNode.parentElement, '[role="treeitem"]'),
        ];

        (event.currentTarget as HTMLElement).tabIndex =
          nodeTouched === false ? -1 : 0;

        if (
          elements.every((element) => (element as HTMLElement).tabIndex === -1)
        ) {
          (elements[0] as HTMLElement).tabIndex = 0;
        }
      }
    },
    focusRowChange(
      firstFocusableRow: HTMLElement,
      currentFocusedRow: HTMLElement,
      lastVisibleDescendant: HTMLElement | null = null
    ) {
      firstFocusableRow.tabIndex = -1;
      currentFocusedRow.tabIndex = 0;

      this.focusNode(lastVisibleDescendant || currentFocusedRow);
    },
    findBeforeClickableNode(node: Element): Element | null {
      const parentListElement = node.closest("ul")?.closest("li");

      if (parentListElement) {
        const prevNodeButton = findSingle(parentListElement, "button");

        if (
          prevNodeButton &&
          (prevNodeButton as HTMLElement).style.visibility !== "hidden"
        ) {
          return parentListElement;
        }

        return this.findBeforeClickableNode(node.previousElementSibling);
      }

      return null;
    },
    toggleCheckbox() {
      const _selectionKeys = this.selectionKeys
        ? { ...this.selectionKeys }
        : {};
      const _check = !this.checked;

      this.propagateDown(this.node, _check, _selectionKeys);

      this.$emit("checkbox-change", {
        node: this.node,
        check: _check,
        selectionKeys: _selectionKeys,
      });
    },
    propagateDown(
      node: TreeNode,
      check: boolean,
      selectionKeys: TreeSelectionKeys
    ) {
      if (check && node.selectable !== false)
        selectionKeys[node.key] = { checked: true, partialChecked: false };
      else delete selectionKeys[node.key];

      if (node.children?.length) {
        for (const child of node.children) {
          this.propagateDown(child, check, selectionKeys);
        }
      }
    },
    propagateUp(event) {
      const check = event.check;
      const _selectionKeys = { ...event.selectionKeys };
      let checkedChildCount = 0;
      let childPartialSelected = false;

      for (let child of this.node.children) {
        if (_selectionKeys[child.key] && _selectionKeys[child.key].checked)
          checkedChildCount++;
        else if (
          _selectionKeys[child.key] &&
          _selectionKeys[child.key].partialChecked
        )
          childPartialSelected = true;
      }

      if (check && checkedChildCount === this.node.children.length) {
        _selectionKeys[this.node.key] = {
          checked: true,
          partialChecked: false,
        };
      } else {
        if (!check) {
          delete _selectionKeys[this.node.key];
        }

        if (
          childPartialSelected ||
          (checkedChildCount > 0 &&
            checkedChildCount !== this.node.children.length)
        )
          _selectionKeys[this.node.key] = {
            checked: false,
            partialChecked: true,
          };
        else delete _selectionKeys[this.node.key];
      }

      this.$emit("checkbox-change", {
        node: event.node,
        check: event.check,
        selectionKeys: _selectionKeys,
      });
    },
    onChildCheckboxChange(event: UIEvent) {
      this.$emit("checkbox-change", event);
    },
    findNextSiblingOfAncestor(nodeElement: Element): Element | null {
      const parentNodeElement = this.getParentNodeElement(nodeElement);

      if (parentNodeElement) {
        if (parentNodeElement.nextElementSibling)
          return parentNodeElement.nextElementSibling;
        return this.findNextSiblingOfAncestor(parentNodeElement);
      }
      return null;
    },
    findLastVisibleDescendant(nodeElement: Element): Element {
      const childrenListElement = nodeElement.children[1];

      if (childrenListElement) {
        const lastChildElement =
          childrenListElement.children[childrenListElement.children.length - 1];

        return this.findLastVisibleDescendant(lastChildElement);
      }
      return nodeElement;
    },
    getParentNodeElement(nodeElement: Element) {
      const parentNodeElement = nodeElement.parentElement?.parentElement;

      return getAttribute(parentNodeElement, "role") === "treeitem"
        ? parentNodeElement
        : null;
    },
    focusNode(element: HTMLElement) {
      element.focus();
    },
    isCheckboxSelectionMode() {
      return this.selectionMode === "checkbox";
    },
    isSameNode(event: UIEvent) {
      return (
        event.currentTarget &&
        ((event.currentTarget as Element).isSameNode(event.target) ||
          (event.currentTarget as Element).isSameNode(
            (event.target as Element).closest('[role="treeitem"]')
          ))
      );
    },
    // ========== Drag and Drop BEGIN ==========
    registerDraggable() {
      // Draggable
      this.unregisterDraggable = draggable({
        element: this.$refs.currentNode,
        getInitialData: () => ({
          key: this.node.key,
        }),
        onDragStart: () => {
          this.dragging = true;
        },
        onDrop: () => {
          this.dragging = false;
        },
      });
      // Drop Target
      this.unregisterDropTarget = dropTargetForElements({
        element: this.$refs.currentNode,
        getData: ({ input, element }) => {
          const data = {
            key: this.node.key,
          };
          return attachInstruction(data, {
            input,
            element,
            currentLevel: 1, // TODO
            indentPerLevel: 4,
            mode: "standard",
          });
        },
        canDrop: ({ source }) => {
          return source.data.key !== this.node.key;
        },
        onDrag: (args) => {
          const instruction: Instruction | null = extractInstruction(
            args.self.data
          );
          console.debug("onDrag", args.self.data.key);
          console.debug(instruction);
          this.dropTargetInstruction = instruction;
        },
        onDragLeave: () => {
          this.dropTargetInstruction = null;
        },
        onDrop: () => {
          this.dropTargetInstruction = null;
        },
      });
    },
    // ========== Drag and Drop END ==========
  },
  computed: {
    hasChildren() {
      return this.node.children && this.node.children.length > 0;
    },
    expanded() {
      return this.expandedKeys && this.expandedKeys[this.node.key] === true;
    },
    leaf() {
      return this.node.leaf === false ? false : !this.node.children?.length;
    },
    selectable() {
      return this.node.selectable === false
        ? false
        : this.selectionMode != null;
    },
    selected() {
      return this.selectionMode && this.selectionKeys
        ? this.selectionKeys[this.node.key] === true
        : false;
    },
    checkboxMode() {
      return (
        this.selectionMode === "checkbox" && this.node.selectable !== false
      );
    },
    checked() {
      return this.selectionKeys
        ? this.selectionKeys[this.node.key] &&
            this.selectionKeys[this.node.key].checked
        : false;
    },
    partialChecked() {
      return this.selectionKeys
        ? this.selectionKeys[this.node.key] &&
            this.selectionKeys[this.node.key].partialChecked
        : false;
    },
    ariaChecked() {
      return this.selectionMode === "single" ||
        this.selectionMode === "multiple"
        ? this.selected
        : undefined;
    },
    ariaSelected() {
      return this.checkboxMode ? this.checked : undefined;
    },
  },
  components: {
    Checkbox,
    ChevronDownIcon,
    ChevronRightIcon,
    CheckIcon,
    MinusIcon,
    SpinnerIcon,
    DropIndicator,
  },
  directives: {
    ripple: Ripple,
  },
});
</script>

<style>
.dragging {
  opacity: 0.5;
}
</style>
