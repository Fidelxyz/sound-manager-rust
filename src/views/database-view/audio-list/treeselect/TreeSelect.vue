<template>
  <div
    ref="container"
    :class="cx('root')"
    :style="sx('root')"
    @click="onClick"
    v-bind="ptmi('root')"
  >
    <div
      class="p-hidden-accessible"
      v-bind="ptm('hiddenInputContainer')"
      :data-p-hidden-accessible="true"
    >
      <input
        ref="focusInput"
        :id="inputId"
        type="text"
        role="combobox"
        :class="inputClass"
        :style="inputStyle"
        readonly
        :disabled="disabled"
        :tabindex="!disabled ? tabindex : -1"
        :aria-labelledby="ariaLabelledby"
        :aria-label="ariaLabel"
        aria-haspopup="tree"
        :aria-expanded="overlayVisible"
        :aria-controls="listId"
        @focus="onFocus($event)"
        @blur="onBlur($event)"
        @keydown="onKeyDown($event)"
        v-bind="{ ...inputProps, ...ptm('hiddenInput') }"
      />
    </div>
    <div :class="cx('labelContainer')" v-bind="ptm('labelContainer')">
      <div :class="cx('label')" v-bind="ptm('label')">
        <slot name="value" :value="selectedNodes" :placeholder="placeholder">
          <template v-if="display === 'comma'">
            {{ label || "empty" }}
          </template>
          <template v-else-if="display === 'chip'">
            <template v-if="chipSelectedItems">
              <span>{{ label }}</span>
            </template>
            <template v-else>
              <div
                v-for="node of selectedNodes"
                :key="node.key"
                :class="cx('chipItem')"
                v-bind="ptm('chipItem')"
              >
                <Chip
                  :class="cx('pcChip')"
                  :label="node.label"
                  :unstyled="unstyled"
                  :pt="ptm('pcChip')"
                />
              </div>
              <template v-if="emptyValue">{{
                placeholder || "empty"
              }}</template>
            </template>
          </template>
        </slot>
      </div>
    </div>
    <slot
      v-if="isClearIconVisible"
      name="clearicon"
      :class="cx('clearIcon')"
      :clearCallback="onClearClick"
    >
      <component
        :is="clearIcon ? 'i' : 'TimesIcon'"
        ref="clearIcon"
        :class="[cx('clearIcon'), clearIcon]"
        @click="onClearClick"
        v-bind="ptm('clearIcon')"
        data-pc-section="clearicon"
      />
    </slot>
    <div
      :class="cx('dropdown')"
      role="button"
      aria-haspopup="tree"
      :aria-expanded="overlayVisible"
      v-bind="ptm('dropdown')"
    >
      <!-- TODO: triggericon is deprecated since v4.0 -->
      <slot
        :name="$slots.dropdownicon ? 'dropdownicon' : 'triggericon'"
        :class="cx('dropdownIcon')"
      >
        <component
          :is="'ChevronDownIcon'"
          :class="cx('dropdownIcon')"
          v-bind="ptm('dropdownIcon')"
        />
      </slot>
    </div>
    <Portal :appendTo="appendTo">
      <transition
        name="p-connected-overlay"
        @enter="onOverlayEnter"
        @after-enter="onOverlayAfterEnter"
        @leave="onOverlayLeave"
        @after-leave="onOverlayAfterLeave"
        v-bind="ptm('transition')"
      >
        <div
          v-if="overlayVisible"
          :ref="overlayRef"
          @click="onOverlayClick"
          :class="[cx('panel'), panelClass]"
          @keydown="onOverlayKeydown"
          v-bind="{ ...panelProps, ...ptm('panel') }"
        >
          <span
            ref="firstHiddenFocusableElementOnOverlay"
            role="presentation"
            class="p-hidden-accessible p-hidden-focusable"
            :tabindex="0"
            @focus="onFirstHiddenFocus"
            v-bind="ptm('hiddenFirstFocusableEl')"
            :data-p-hidden-accessible="true"
            :data-p-hidden-focusable="true"
          ></span>
          <slot name="header" :value="d_value" :options="options"></slot>
          <div
            :class="cx('treeContainer')"
            :style="{ 'max-height': scrollHeight }"
            v-bind="ptm('treeContainer')"
          >
            <TSTree
              ref="tree"
              :id="listId"
              :value="options"
              :selectionMode="selectionMode"
              :loading="loading"
              :loadingIcon="loadingIcon"
              :loadingMode="loadingMode"
              :filter="filter"
              :filterBy="filterBy"
              :filterMode="filterMode"
              :filterPlaceholder="filterPlaceholder"
              :filterLocale="filterLocale"
              @update:selectionKeys="onSelectionChange"
              :selectionKeys="d_value"
              :expandedKeys="d_expandedKeys"
              @update:expandedKeys="onNodeToggle"
              :metaKeySelection="metaKeySelection"
              @node-expand="$emit('node-expand', $event)"
              @node-collapse="$emit('node-collapse', $event)"
              @node-select="onNodeSelect"
              @node-unselect="onNodeUnselect"
              @click.stop
              :level="0"
              :unstyled="unstyled"
              :pt="ptm('pcTree')"
            >
              <template v-if="$slots.option" #default="optionSlotProps">
                <slot
                  name="option"
                  :node="optionSlotProps.node"
                  :expanded="optionSlotProps.expanded"
                  :selected="optionSlotProps.selected"
                />
              </template>
              <template
                v-if="$slots.itemtoggleicon"
                #toggleicon="iconSlotProps"
              >
                <slot
                  name="itemtoggleicon"
                  :node="iconSlotProps.node"
                  :expanded="iconSlotProps.expanded"
                  :class="iconSlotProps.class"
                />
              </template>
              <!--TODO: itemtogglericon deprecated since v4.0-->
              <template
                v-else-if="$slots.itemtogglericon"
                #togglericon="iconSlotProps"
              >
                <slot
                  name="itemtogglericon"
                  :node="iconSlotProps.node"
                  :expanded="iconSlotProps.expanded"
                  :class="iconSlotProps.class"
                />
              </template>
              <template
                v-if="$slots.itemcheckboxicon"
                #checkboxicon="iconSlotProps"
              >
                <slot
                  name="itemcheckboxicon"
                  :checked="iconSlotProps.checked"
                  :partialChecked="iconSlotProps.partialChecked"
                  :class="iconSlotProps.class"
                />
              </template>
            </TSTree>
            <div
              v-if="emptyOptions && !loading"
              :class="cx('emptyMessage')"
              v-bind="ptm('emptyMessage')"
            >
              <slot name="empty">{{ emptyMessageText }}</slot>
            </div>
          </div>
          <slot name="footer" :value="d_value" :options="options"></slot>
          <span
            ref="lastHiddenFocusableElementOnOverlay"
            role="presentation"
            class="p-hidden-accessible p-hidden-focusable"
            :tabindex="0"
            @focus="onLastHiddenFocus"
            v-bind="ptm('hiddenLastFocusableEl')"
            :data-p-hidden-accessible="true"
            :data-p-hidden-focusable="true"
          ></span>
        </div>
      </transition>
    </Portal>
  </div>
</template>

<script>
import {
  absolutePosition,
  addStyle,
  find,
  findSingle,
  focus,
  getFirstFocusableElement,
  getFocusableElements,
  getLastFocusableElement,
  getOuterWidth,
  isTouchDevice,
  relativePosition,
} from "@primeuix/utils/dom";
import { isEmpty, isNotEmpty } from "@primeuix/utils/object";
import { ZIndex } from "@primeuix/utils/zindex";
import { ConnectedOverlayScrollHandler } from "@primevue/core/utils";
import ChevronDownIcon from "@primevue/icons/chevrondown";
import TimesIcon from "@primevue/icons/times";
import Chip from "primevue/chip";
import OverlayEventBus from "primevue/overlayeventbus";
import Portal from "primevue/portal";
import Ripple from "primevue/ripple";
import Tree from "../tree";
import BaseTreeSelect from "./BaseTreeSelect.vue";

export default {
  name: "TreeSelect",
  extends: BaseTreeSelect,
  inheritAttrs: false,
  emits: [
    "before-show",
    "before-hide",
    "change",
    "show",
    "hide",
    "node-select",
    "node-unselect",
    "node-expand",
    "node-collapse",
    "focus",
    "blur",
    "update:expandedKeys",
  ],
  inject: {
    $pcFluid: { default: null },
  },
  data() {
    return {
      focused: false,
      overlayVisible: false,
      d_expandedKeys: this.expandedKeys || {},
    };
  },
  watch: {
    modelValue: {
      handler: function () {
        if (!this.selfChange) {
          this.updateTreeState();
        }

        this.selfChange = false;
      },
      immediate: true,
    },
    options() {
      this.updateTreeState();
    },
    expandedKeys(value) {
      this.d_expandedKeys = value;
    },
  },
  outsideClickListener: null,
  resizeListener: null,
  scrollHandler: null,
  overlay: null,
  selfChange: false,
  selfClick: false,
  beforeUnmount() {
    this.unbindOutsideClickListener();
    this.unbindResizeListener();

    if (this.scrollHandler) {
      this.scrollHandler.destroy();
      this.scrollHandler = null;
    }

    if (this.overlay) {
      ZIndex.clear(this.overlay);
      this.overlay = null;
    }
  },
  mounted() {
    this.updateTreeState();
  },
  methods: {
    show() {
      this.$emit("before-show");
      this.overlayVisible = true;
    },
    hide() {
      this.$emit("before-hide");
      this.overlayVisible = false;
      this.$refs.focusInput.focus();
    },
    onFocus(event) {
      this.focused = true;
      this.$emit("focus", event);
    },
    onBlur(event) {
      this.focused = false;
      this.$emit("blur", event);
      this.formField.onBlur?.();
    },
    onClick(event) {
      if (this.disabled) {
        return;
      }

      if (
        event.target.tagName === "INPUT" ||
        event.target.getAttribute("data-pc-section") === "clearicon" ||
        event.target.closest('[data-pc-section="clearicon"]')
      ) {
        return;
      } else if (!this.overlay || !this.overlay.contains(event.target)) {
        if (this.overlayVisible) this.hide();
        else this.show();

        focus(this.$refs.focusInput);
      }
    },
    onClearClick() {
      this.onSelectionChange(null);
    },
    onSelectionChange(keys) {
      this.selfChange = true;
      this.writeValue(keys);
      this.$emit("change", keys);
    },
    onNodeSelect(node) {
      this.$emit("node-select", node);

      if (this.selectionMode === "single") {
        this.hide();
      }
    },
    onNodeUnselect(node) {
      this.$emit("node-unselect", node);
    },
    onNodeToggle(keys) {
      this.d_expandedKeys = keys;

      this.$emit("update:expandedKeys", this.d_expandedKeys);
    },
    getSelectedItemsLabel() {
      let pattern = /{(.*?)}/;
      const selectedItemsLabel =
        this.selectedItemsLabel ||
        this.$primevue.config.locale.selectionMessage;

      if (pattern.test(selectedItemsLabel)) {
        return selectedItemsLabel.replace(
          selectedItemsLabel.match(pattern)[0],
          Object.keys(this.d_value).length + "",
        );
      }

      return selectedItemsLabel;
    },
    onFirstHiddenFocus(event) {
      const focusableEl =
        event.relatedTarget === this.$refs.focusInput
          ? getFirstFocusableElement(
              this.overlay,
              ':not([data-p-hidden-focusable="true"])',
            )
          : this.$refs.focusInput;

      focus(focusableEl);
    },
    onLastHiddenFocus(event) {
      const focusableEl =
        event.relatedTarget === this.$refs.focusInput
          ? getLastFocusableElement(
              this.overlay,
              ':not([data-p-hidden-focusable="true"])',
            )
          : this.$refs.focusInput;

      focus(focusableEl);
    },
    onKeyDown(event) {
      switch (event.code) {
        case "ArrowDown":
          this.onArrowDownKey(event);
          break;

        case "Space":
        case "Enter":
        case "NumpadEnter":
          this.onEnterKey(event);
          break;

        case "Escape":
          this.onEscapeKey(event);
          break;

        case "Tab":
          this.onTabKey(event);
          break;

        default:
          break;
      }
    },
    onArrowDownKey(event) {
      if (this.overlayVisible) return;

      this.show();

      this.$nextTick(() => {
        const treeNodeEl = find(
          this.$refs.tree.$el,
          '[data-pc-section="treeitem"]',
        );
        const focusedElement = [...treeNodeEl].find(
          (item) => item.getAttribute("tabindex") === "0",
        );

        focus(focusedElement);
      });

      event.preventDefault();
    },
    onEnterKey(event) {
      if (this.overlayVisible) {
        this.hide();
      } else {
        this.onArrowDownKey(event);
      }

      event.preventDefault();
    },
    onEscapeKey(event) {
      if (this.overlayVisible) {
        this.hide();
        event.preventDefault();
      }
    },
    onTabKey(event, pressedInInputText = false) {
      if (!pressedInInputText) {
        if (this.overlayVisible && this.hasFocusableElements()) {
          focus(this.$refs.firstHiddenFocusableElementOnOverlay);

          event.preventDefault();
        }
      }
    },
    hasFocusableElements() {
      return (
        getFocusableElements(
          this.overlay,
          ':not([data-p-hidden-focusable="true"])',
        ).length > 0
      );
    },
    onOverlayEnter(el) {
      ZIndex.set("overlay", el, this.$primevue.config.zIndex.overlay);

      addStyle(el, { position: "absolute", top: "0" });
      this.alignOverlay();
      this.focus();

      // Issue: #7508
      this.$attrSelector && el.setAttribute(this.$attrSelector, "");
    },
    onOverlayAfterEnter() {
      this.bindOutsideClickListener();
      this.bindScrollListener();
      this.bindResizeListener();
      this.scrollValueInView();
      this.$emit("show");
    },
    onOverlayLeave() {
      this.unbindOutsideClickListener();
      this.unbindScrollListener();
      this.unbindResizeListener();
      this.$emit("hide");
      this.overlay = null;
    },
    onOverlayAfterLeave(el) {
      ZIndex.clear(el);
    },
    focus() {
      let focusableElements = getFocusableElements(this.overlay);

      if (focusableElements && focusableElements.length > 0) {
        focusableElements[0].focus();
      }
    },
    alignOverlay() {
      if (this.appendTo === "self") {
        relativePosition(this.overlay, this.$el);
      } else {
        this.overlay.style.minWidth = getOuterWidth(this.$el) + "px";
        absolutePosition(this.overlay, this.$el);
      }
    },
    bindOutsideClickListener() {
      if (!this.outsideClickListener) {
        this.outsideClickListener = (event) => {
          if (
            this.overlayVisible &&
            !this.selfClick &&
            this.isOutsideClicked(event)
          ) {
            this.hide();
          }

          this.selfClick = false;
        };

        document.addEventListener("click", this.outsideClickListener, true);
      }
    },
    unbindOutsideClickListener() {
      if (this.outsideClickListener) {
        document.removeEventListener("click", this.outsideClickListener, true);
        this.outsideClickListener = null;
      }
    },
    bindScrollListener() {
      if (!this.scrollHandler) {
        this.scrollHandler = new ConnectedOverlayScrollHandler(
          this.$refs.container,
          () => {
            if (this.overlayVisible) {
              this.hide();
            }
          },
        );
      }

      this.scrollHandler.bindScrollListener();
    },
    unbindScrollListener() {
      if (this.scrollHandler) {
        this.scrollHandler.unbindScrollListener();
      }
    },
    bindResizeListener() {
      if (!this.resizeListener) {
        this.resizeListener = () => {
          if (this.overlayVisible && !isTouchDevice()) {
            this.hide();
          }
        };

        window.addEventListener("resize", this.resizeListener);
      }
    },
    unbindResizeListener() {
      if (this.resizeListener) {
        window.removeEventListener("resize", this.resizeListener);
        this.resizeListener = null;
      }
    },
    isOutsideClicked(event) {
      return !(
        this.$el.isSameNode(event.target) ||
        this.$el.contains(event.target) ||
        (this.overlay && this.overlay.contains(event.target))
      );
    },
    overlayRef(el) {
      this.overlay = el;
    },
    onOverlayClick(event) {
      OverlayEventBus.emit("overlay-click", {
        originalEvent: event,
        target: this.$el,
      });

      this.selfClick = true;
    },
    onOverlayKeydown(event) {
      if (event.code === "Escape") this.hide();
    },
    fillNodeMap(node, nodeMap) {
      nodeMap[node.key] = node;

      if (node.children?.length) {
        node.children.forEach((children) =>
          this.fillNodeMap(children, nodeMap),
        );
      }
    },
    isSelected(node, keys) {
      return this.selectionMode === "checkbox"
        ? keys[node.key] && keys[node.key].checked
        : keys[node.key];
    },
    updateTreeState() {
      let keys = { ...this.d_value };

      if (keys && this.options) {
        this.updateTreeBranchState(null, null, keys);
      }
    },
    updateTreeBranchState(node, path, keys) {
      if (node) {
        if (this.isSelected(node, keys)) {
          this.expandPath(path);
          delete keys[node.key];
        }

        if (Object.keys(keys).length && node.children) {
          for (let childNode of node.children) {
            path.push(node.key);
            this.updateTreeBranchState(childNode, path, keys);
          }
        }
      } else {
        for (let childNode of this.options) {
          this.updateTreeBranchState(childNode, [], keys);
        }
      }
    },
    expandPath(path) {
      if (path.length > 0) {
        for (let key of path) {
          this.d_expandedKeys[key] = true;
        }

        this.d_expandedKeys = { ...this.d_expandedKeys };
        this.$emit("update:expandedKeys", this.d_expandedKeys);
      }
    },
    scrollValueInView() {
      if (this.overlay) {
        let selectedItem = findSingle(this.overlay, '[data-p-selected="true"]');

        if (selectedItem) {
          selectedItem.scrollIntoView({ block: "nearest", inline: "start" });
        }
      }
    },
  },
  computed: {
    nodeMap() {
      const nodeMap = {};

      this.options?.forEach((node) => this.fillNodeMap(node, nodeMap));

      return nodeMap;
    },
    selectedNodes() {
      let selectedNodes = [];

      if (this.d_value && this.options) {
        Object.keys(this.d_value).forEach((key) => {
          const node = this.nodeMap[key];

          if (this.isSelected(node, this.d_value)) {
            selectedNodes.push(node);
          }
        });
      }

      return selectedNodes;
    },
    label() {
      let value = this.selectedNodes;
      let label;

      if (value.length) {
        if (
          isNotEmpty(this.maxSelectedLabels) &&
          value.length > this.maxSelectedLabels
        ) {
          label = this.getSelectedItemsLabel();
        } else {
          label = value.map((node) => node.label).join(", ");
        }
      } else {
        label = this.placeholder;
      }

      return label;
    },
    chipSelectedItems() {
      return (
        isNotEmpty(this.maxSelectedLabels) &&
        this.d_value &&
        Object.keys(this.d_value).length > this.maxSelectedLabels
      );
    },
    emptyMessageText() {
      return this.emptyMessage || this.$primevue.config.locale.emptyMessage;
    },
    emptyValue() {
      return !this.$filled;
    },
    emptyOptions() {
      return !this.options || this.options.length === 0;
    },
    listId() {
      return this.$id + "_list";
    },
    hasFluid() {
      return isEmpty(this.fluid) ? !!this.$pcFluid : this.fluid;
    },
    isClearIconVisible() {
      return this.showClear && this.d_value != null && isNotEmpty(this.options);
    },
  },
  components: {
    TSTree: Tree,
    Chip,
    Portal,
    ChevronDownIcon,
    TimesIcon,
  },
  directives: {
    ripple: Ripple,
  },
};
</script>
