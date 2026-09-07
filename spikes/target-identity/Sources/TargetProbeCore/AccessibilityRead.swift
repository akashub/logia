import ApplicationServices

public struct CaptureFailure: Error, CustomStringConvertible {
    public let description: String
    let code: AXError?

    init(_ stage: String, code: AXError? = nil) {
        self.code = code
        description = code.map { "\(stage):ax-error-\($0.rawValue)" } ?? stage
    }
}

enum AccessibilityRead {
    static func value(_ element: AXUIElement, _ attribute: String) throws -> CFTypeRef {
        var result: CFTypeRef?
        let code = AXUIElementCopyAttributeValue(element, attribute as CFString, &result)
        guard code == .success else { throw CaptureFailure(attribute, code: code) }
        guard let result else { throw CaptureFailure(attribute, code: .noValue) }
        return result
    }

    static func element(_ parent: AXUIElement, _ attribute: String) throws -> AXUIElement {
        let result = try value(parent, attribute)
        guard CFGetTypeID(result) == AXUIElementGetTypeID() else {
            throw CaptureFailure("\(attribute):unexpected-type")
        }
        return unsafeDowncast(result, to: AXUIElement.self)
    }

    static func requireEditable(_ field: AXUIElement) throws {
        guard let role = try value(field, kAXRoleAttribute) as? String,
              role == kAXTextFieldRole || role == kAXTextAreaRole else {
            throw CaptureFailure("not-a-text-field")
        }
        do {
            let subrole = try value(field, kAXSubroleAttribute)
            guard let name = subrole as? String else { throw CaptureFailure("subrole:unexpected-type") }
            if name == kAXSecureTextFieldSubrole { throw CaptureFailure("secure-field") }
        } catch let error as CaptureFailure where error.code == .noValue || error.code == .attributeUnsupported {
            // Ordinary text areas legitimately have no subrole. Transport errors
            // still propagate; absence is not interchangeable with a failed read.
        }
        var settable: DarwinBoolean = false
        let code = AXUIElementIsAttributeSettable(field, kAXValueAttribute as CFString, &settable)
        guard code == .success else { throw CaptureFailure("editable-capability", code: code) }
        guard settable.boolValue else { throw CaptureFailure("value-not-settable") }
    }
}
