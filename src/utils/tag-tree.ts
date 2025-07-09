import { TagNode } from "@/types";

/**
 * Take `node` from `tree`.
 */
export function takeNode(node: TagNode, tree: TagNode[]): TagNode | null {
  for (let i = 0; i < tree.length; i++) {
    const current = tree[i];
    if (current === node) {
      return tree.splice(i, 1)[0];
    }

    if (current.children) {
      const taked = takeNode(node, current.children);
      if (taked) return taked;
    }
  }

  return null;
}

type InsertNodeResult = {
  parent: TagNode | null;
  location: number;
};

/**
 * Insert `node` above `targetNode` in `tree`.
 */
export function insertNodeAbove(
  node: TagNode,
  targetNode: TagNode,
  tree: TagNode[],
): InsertNodeResult | null {
  const insertNodeAbove = (
    tree: TagNode[],
    parent: TagNode | null = null,
  ): InsertNodeResult | null => {
    for (let i = 0; i < tree.length; i++) {
      const current = tree[i];
      if (current === targetNode) {
        tree.splice(i, 0, node);
        return { parent, location: i };
      }

      if (current.children) {
        const inserted = insertNodeAbove(current.children, current);
        if (inserted) return inserted;
      }
    }

    return null;
  };
  return insertNodeAbove(tree);
}

/**
 * Insert `node` below `targetNode` in `tree`.
 */
export function insertNodeBelow(
  node: TagNode,
  targetNode: TagNode,
  tree: TagNode[],
): InsertNodeResult | null {
  const insertNodeBelow = (
    tree: TagNode[],
    parent: TagNode | null = null,
  ): InsertNodeResult | null => {
    for (let i = 0; i < tree.length; i++) {
      const current = tree[i];
      if (current === targetNode) {
        tree.splice(i + 1, 0, node);
        return { parent, location: i + 1 };
      }

      if (current.children) {
        const inserted = insertNodeBelow(current.children, current);
        if (inserted) return inserted;
      }
    }

    return null;
  };
  return insertNodeBelow(tree);
}

/**
 * Append `node` as a child of `targetNode` in `tree`.
 */
export function appendChildNode(
  node: TagNode,
  targetNode: TagNode,
  tree: TagNode[],
): InsertNodeResult | null {
  const appendChildNode = (tree: TagNode[]): InsertNodeResult | null => {
    for (let i = 0; i < tree.length; i++) {
      const current = tree[i];
      if (current === targetNode) {
        current.children.push(node);
        return { parent: current, location: current.children.length - 1 };
      }

      if (current.children) {
        const inserted = appendChildNode(current.children);
        if (inserted) return inserted;
      }
    }

    return null;
  };
  return appendChildNode(tree);
}
