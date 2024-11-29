use xml::attribute::OwnedAttribute;

pub(crate) fn get_id(attributes: Vec<OwnedAttribute>, match_string: String) -> Option<String> {
    attributes
        .into_iter()
        .filter(|x| x.name.local_name == match_string)
        .map(|x| x.value)
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
