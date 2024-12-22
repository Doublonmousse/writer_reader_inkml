use xml::attribute::OwnedAttribute;

pub(crate) fn get_id(attributes: &[OwnedAttribute], match_string: String) -> Option<String> {
    attributes
        .iter()
        .filter(|x| x.name.local_name == match_string)
        .map(|x| x.value.clone())
        .next()
}

/// gets the attributes we asked for in that order
pub(crate) fn get_ids(
    attributes: Vec<OwnedAttribute>,
    match_string: Vec<String>,
) -> Vec<Option<String>> {
    match_string
        .into_iter()
        .map(|x| {
            attributes
                .clone()
                .into_iter()
                .filter(|attr| x == attr.name.local_name)
                .map(|x| x.value.clone())
                .next()
        })
        .collect()
}

pub(crate) fn verify_channel_properties(ids: &[Option<String>]) -> bool {
    if ids.iter().all(|new| new.is_some()) {
        // we have verified all of the information is there
        // according to the spec, channel name and value are required
        // and units is optional. However, this is required (?) if the
        // property name is "resolution" (and that's the only tag we will read for now)
        if let Some(property_name) = &ids[1] {
            if property_name.as_str() == "resolution" {
                return true;
            }
        }
        return false;
    }
    false
}
