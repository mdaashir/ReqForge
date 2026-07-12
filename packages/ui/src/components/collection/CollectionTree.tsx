import * as React from 'react'
import {
  DndContext,
  closestCenter,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
} from '@dnd-kit/core'
import {
  SortableContext,
  arrayMove,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
} from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import { useVirtualizer } from '@tanstack/react-virtual'
import {
  ChevronDown,
  ChevronRight,
  Folder,
  FolderOpen,
  FileText,
  Plus,
  GripVertical,
} from 'lucide-react'
import { cn } from '../../lib/utils'
import { Button } from '../primitives/Button'
import type { Collection, CollectionItem } from '../../types'

export interface CollectionTreeProps {
  collections: Collection[]
  selectedRequestId?: string
  onSelectRequest: (collectionId: string, requestId: string) => void
  /** Called when the user finishes a drag. The parent should persist the
   *  new order. */
  onReorder?: (collectionId: string, items: CollectionItem[]) => void
  onCreateRequest?: (collectionId: string) => void
  onCreateFolder?: (collectionId: string) => void
  className?: string
}

interface SortableItemProps {
  item: CollectionItem
  collectionId: string
  depth: number
  selectedRequestId?: string
  onSelectRequest: (collectionId: string, requestId: string) => void
  onCreateRequest?: (collectionId: string) => void
}

function SortableRequestItem({
  item,
  collectionId,
  depth,
  selectedRequestId,
  onSelectRequest,
}: SortableItemProps) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } =
    useSortable({ id: item.id })

  const style: React.CSSProperties = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.5 : 1,
  }

  const method = item.request?.method || 'GET'
  const methodColor: Record<string, string> = {
    GET: 'text-green-600 dark:text-green-400',
    POST: 'text-orange-600 dark:text-orange-400',
    PUT: 'text-blue-600 dark:text-blue-400',
    PATCH: 'text-purple-600 dark:text-purple-400',
    DELETE: 'text-red-600 dark:text-red-400',
  }
  const isSelected = selectedRequestId === item.id

  return (
    <div
      ref={setNodeRef}
      style={style}
      className={cn(
        'group flex items-center gap-1 w-full pr-2 py-1 text-sm rounded text-left',
        isSelected
          ? 'bg-blue-100 dark:bg-blue-900/30'
          : 'hover:bg-gray-100 dark:hover:bg-gray-800',
        isDragging && 'shadow-lg z-50 bg-white dark:bg-gray-800'
      )}
    >
      <button
        {...attributes}
        {...listeners}
        className="cursor-grab opacity-0 group-hover:opacity-100 text-gray-400 hover:text-gray-600 px-1"
        title="Drag to reorder"
        data-testid={`collection-drag-handle-${item.id}`}
      >
        <GripVertical className="h-3 w-3" />
      </button>
      <button
        onClick={() => onSelectRequest(collectionId, item.id)}
        className="flex items-center gap-2 flex-1 min-w-0"
        style={{ paddingLeft: `${depth * 12 + 16}px` }}
        data-testid={`collection-request-${item.id}`}
      >
        <FileText className="h-4 w-4 text-gray-500 flex-shrink-0" />
        <span
          className={cn(
            'font-mono font-semibold text-xs flex-shrink-0',
            methodColor[method] || 'text-gray-600'
          )}
        >
          {method}
        </span>
        <span className="truncate text-gray-700 dark:text-gray-300">
          {item.name}
        </span>
      </button>
    </div>
  )
}

function SortableFolderItem({
  item,
  collectionId,
  depth,
  selectedRequestId,
  onSelectRequest,
  onCreateRequest,
}: SortableItemProps) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } =
    useSortable({ id: item.id })

  const [open, setOpen] = React.useState(true)

  const style: React.CSSProperties = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.5 : 1,
  }

  return (
    <div ref={setNodeRef} style={style}>
      <div
        className={cn(
          'group flex items-center gap-1 w-full px-2 py-1 text-sm rounded text-left hover:bg-gray-100 dark:hover:bg-gray-800',
          isDragging && 'shadow-lg bg-white dark:bg-gray-800'
        )}
        style={{ paddingLeft: `${depth * 12 + 8}px` }}
      >
        <button
          {...attributes}
          {...listeners}
          className="cursor-grab opacity-0 group-hover:opacity-100 text-gray-400 hover:text-gray-600"
          title="Drag to reorder"
          data-testid={`collection-drag-handle-${item.id}`}
        >
          <GripVertical className="h-3 w-3" />
        </button>
        <button
          onClick={() => setOpen(!open)}
          className="flex items-center gap-1 flex-1 min-w-0"
          aria-expanded={open}
        >
          {open ? (
            <ChevronDown className="h-3 w-3 text-gray-500 flex-shrink-0" />
          ) : (
            <ChevronRight className="h-3 w-3 text-gray-500 flex-shrink-0" />
          )}
          {open ? (
            <FolderOpen className="h-4 w-4 text-yellow-500 flex-shrink-0" />
          ) : (
            <Folder className="h-4 w-4 text-yellow-500 flex-shrink-0" />
          )}
          <span className="truncate">{item.name}</span>
        </button>
      </div>
      {open && (
        <SortableContext
          items={(item.children ?? []).map((c) => c.id)}
          strategy={verticalListSortingStrategy}
        >
          <div>
            {(item.children ?? []).map((child) => (
              <SortableTreeItem
                key={child.id}
                item={child}
                collectionId={collectionId}
                depth={depth + 1}
                selectedRequestId={selectedRequestId}
                onSelectRequest={onSelectRequest}
                onCreateRequest={onCreateRequest}
              />
            ))}
          </div>
        </SortableContext>
      )}
    </div>
  )
}

/// Virtualized list of tree items to handle 10k+ collections smoothly
const VirtualizedItemList = React.memo(function VirtualizedItemList({
  items,
  collectionId,
  depth,
  selectedRequestId,
  onSelectRequest,
  onCreateRequest,
}: {
  items: CollectionItem[]
  collectionId: string
  depth: number
  selectedRequestId: string | null
  onSelectRequest: (requestId: string) => void
  onCreateRequest: (parentId: string) => void
}) {
  const parentRef = React.useRef<HTMLDivElement>(null)
  const virtualizer = useVirtualizer({
    count: items.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 34,
    overscan: 10,
  })

  if (items.length === 0) {
    return (
      <div className="p-2 text-xs text-gray-400 text-center italic">
        Empty collection
      </div>
    )
  }

  return (
    <div
      ref={parentRef}
      className="relative"
      style={{ height: virtualizer.getTotalSize() }}
    >
      {virtualizer.getVirtualItems().map((virtualRow) => {
        const item = items[virtualRow.index]
        return (
          <div
            key={item.id}
            style={{
              position: 'absolute',
              top: 0,
              left: 0,
              width: '100%',
              height: `${virtualRow.size}px`,
              transform: `translateY(${virtualRow.start}px)`,
            }}
          >
            <SortableTreeItem
              item={item}
              collectionId={collectionId}
              depth={depth}
              selectedRequestId={selectedRequestId}
              onSelectRequest={onSelectRequest}
              onCreateRequest={onCreateRequest}
            />
          </div>
        )
      })}
    </div>
  )
})

function SortableTreeItem(props: SortableItemProps) {
  if (props.item.type === 'folder') {
    return <SortableFolderItem {...props} />
  }
  return <SortableRequestItem {...props} />
}

export const CollectionTree = React.forwardRef<HTMLDivElement, CollectionTreeProps>(
  (
    {
      collections,
      selectedRequestId,
      onSelectRequest,
      onReorder,
      onCreateRequest,
      className,
    },
    ref
  ) => {
    const sensors = useSensors(
      useSensor(PointerSensor, { activationConstraint: { distance: 4 } }),
      useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates })
    )

    const handleDragEnd = (collectionId: string) => (event: DragEndEvent) => {
      const { active, over } = event
      if (!over || active.id === over.id) return
      if (!onReorder) return

      const collection = collections.find((c) => c.id === collectionId)
      if (!collection) return

      const oldIndex = collection.items.findIndex((i) => i.id === active.id)
      const newIndex = collection.items.findIndex((i) => i.id === over.id)
      if (oldIndex === -1 || newIndex === -1) return

      const newItems = arrayMove(collection.items, oldIndex, newIndex)
      onReorder(collectionId, newItems)
    }

    return (
      <div
        ref={ref}
        className={cn('flex flex-col h-full overflow-auto', className)}
        data-testid="collection-tree"
      >
        {collections.length === 0 ? (
          <div className="p-4 text-sm text-gray-500 dark:text-gray-400 text-center">
            No collections. Import or create one to get started.
          </div>
        ) : (
          collections.map((collection) => (
            <div key={collection.id} className="mb-3" data-testid={`collection-${collection.id}`}>
              <div className="flex items-center justify-between px-2 py-1.5 border-b border-gray-200 dark:border-gray-700">
                <span className="text-xs font-semibold uppercase tracking-wider text-gray-500 dark:text-gray-400 truncate">
                  {collection.name}
                </span>
                {onCreateRequest && (
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={() => onCreateRequest(collection.id)}
                    title="Add request"
                    data-testid={`collection-add-${collection.id}`}
                  >
                    <Plus className="h-3 w-3" />
                  </Button>
                )}
              </div>

              <DndContext
                sensors={sensors}
                collisionDetection={closestCenter}
                onDragEnd={handleDragEnd(collection.id)}
              >
                <SortableContext
                  items={collection.items.map((i) => i.id)}
                  strategy={verticalListSortingStrategy}
                >
                  <VirtualizedItemList
                    items={collection.items}
                    collectionId={collection.id}
                    depth={0}
                    selectedRequestId={selectedRequestId}
                    onSelectRequest={onSelectRequest}
                    onCreateRequest={onCreateRequest}
                  />
                </SortableContext>
              </DndContext>
            </div>
          ))
        )}
      </div>
    )
  }
)

CollectionTree.displayName = 'CollectionTree'
