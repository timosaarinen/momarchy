local ui = {}

local function element(kind, value)
  value.__momarchy_ui = kind
  return value
end

function ui.title(text)
  return element("title", { text = text })
end

function ui.subtitle(text)
  return element("subtitle", { text = text })
end

function ui.text(text)
  return element("text", { text = text })
end

function ui.button(id, label, hint, action)
  return element("button", {
    id = id,
    label = label,
    hint = hint,
    action = action,
  })
end

function ui.go(screen)
  return { screen = screen }
end

function ui.message(text)
  return { message = text }
end

function ui.open(target, live_message)
  return {
    open = target,
    live_message = live_message,
  }
end

function ui.run(command, kind, live_message)
  return {
    command = command,
    kind = kind,
    live_message = live_message,
  }
end

function ui.screen(elements)
  local screen = {
    title = nil,
    subtitle = "",
    body = nil,
    buttons = {},
  }

  for _, item in ipairs(elements) do
    local kind = type(item) == "table" and item.__momarchy_ui or nil

    if kind == "title" then
      assert(screen.title == nil, "screen may contain only one ui.title")
      screen.title = item.text
    elseif kind == "subtitle" then
      screen.subtitle = item.text
    elseif kind == "text" then
      assert(screen.body == nil, "screen may contain only one ui.text")
      screen.body = item.text
    elseif kind == "button" then
      table.insert(screen.buttons, {
        id = item.id,
        label = item.label,
        hint = item.hint,
        action = item.action,
      })
    else
      error("ui.screen contains an unknown element")
    end
  end

  assert(screen.title ~= nil, "screen requires ui.title")
  return screen
end

function ui.app(spec)
  spec.version = spec.version or 1
  return spec
end

return ui
