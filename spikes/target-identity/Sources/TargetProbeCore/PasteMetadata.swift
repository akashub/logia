import ApplicationServices

// Read metadata only. Unsupported is absence; failures and malformed attributes
// are not permission to weaken the delivery check.
enum PasteMetadata {
    static func optional(_ element: AXUIElement, _ name: String) throws -> CFTypeRef? {
        do { return try AccessibilityRead.value(element, name) }
        catch let error as CaptureFailure where error.code == .noValue || error.code == .attributeUnsupported { return nil }
    }

    static func element(_ parent: AXUIElement, _ name: String) throws -> AXUIElement? {
        guard let value = try optional(parent, name) else { return nil }
        guard CFGetTypeID(value) == AXUIElementGetTypeID() else { throw CaptureFailure("paste-element-type") }
        return unsafeDowncast(value, to: AXUIElement.self)
    }

    static func string(_ element: AXUIElement, _ name: String) throws -> String? {
        guard let value = try optional(element, name) else { return nil }
        guard let string = value as? String else { throw CaptureFailure("paste-string-type") }
        return string
    }

    static func bool(_ element: AXUIElement, _ name: String) throws -> Bool? {
        guard let value = try optional(element, name) else { return nil }
        guard CFGetTypeID(value) == CFBooleanGetTypeID() else { throw CaptureFailure("paste-bool-type") }
        return (value as! Bool)
    }

    static func selection(_ field: AXUIElement) throws -> CFRange? {
        guard let value = try optional(field, kAXSelectedTextRangeAttribute) else { return nil }
        guard CFGetTypeID(value) == AXValueGetTypeID() else { throw CaptureFailure("paste-selection-type") }
        let ax = unsafeDowncast(value, to: AXValue.self)
        var range = CFRange()
        guard AXValueGetType(ax) == .cfRange, AXValueGetValue(ax, .cfRange, &range),
              range.location >= 0, range.length >= 0 else { throw CaptureFailure("paste-selection-unavailable") }
        return range
    }

    static func requirePasteCandidate(_ field: AXUIElement) throws {
        let role = try string(field, kAXRoleAttribute)
        let subrole = try string(field, kAXSubroleAttribute)
        guard subrole != kAXSecureTextFieldSubrole else { throw CaptureFailure("secure-field") }
        guard try bool(field, kAXEnabledAttribute) != false else { throw CaptureFailure("disabled-field") }
        // Containers and custom views can own a keyboard editor. A known button,
        // menu, image, etc. is not a text cursor. AX writability is not consulted.
        let candidates = [kAXTextFieldRole, kAXTextAreaRole, kAXComboBoxRole, kAXGroupRole,
                          kAXScrollAreaRole, kAXUnknownRole, "AXWebArea", "AXLayoutArea"]
        guard role == nil || candidates.contains(role!) else { throw CaptureFailure("nontext-destination") }
    }
}
