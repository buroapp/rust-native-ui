import CoreGraphics
import Foundation
let target = CommandLine.arguments[1]
let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly], kCGNullWindowID) as! [[String: Any]]
for w in list {
    if let owner = w[kCGWindowOwnerName as String] as? String, owner == target,
       let layer = w[kCGWindowLayer as String] as? Int, layer == 0,
       let num = w[kCGWindowNumber as String] as? Int {
        print(num); break
    }
}
