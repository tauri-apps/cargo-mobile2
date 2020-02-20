use cocoa::base::id;
use objc::runtime::Object;
use objc_id::Id;

pub unsafe fn raii(obj: id) -> Id<Object> {
    Id::<Object>::from_ptr(obj)
}
