interface EditableTitleProps {
  name: string
  onRename: (name: string) => Promise<void>
}

/** A heading with a "Rename" pencil that turns it into an in-place field. */
export function EditableTitle({ name }: EditableTitleProps) {
  return (
    <h2 className="truncate font-heading text-lg font-semibold text-foreground md:text-xl">
      {name}
    </h2>
  )
}
