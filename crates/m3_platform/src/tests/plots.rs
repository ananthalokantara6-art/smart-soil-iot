use super::*;

fn new_plot(name: &str) -> NewPlot {
    NewPlot {
        name: name.into(),
        latitude: None,
        longitude: None,
        area_ha: None,
        plant: None,
    }
}

#[test]
fn valid_plot_is_trimmed_and_gets_uuid() {
    let meta = PlotMeta::from_new(new_plot("  Petak A  ")).unwrap();
    assert_eq!(meta.name, "Petak A");
    let other = PlotMeta::from_new(new_plot("Petak A")).unwrap();
    assert_ne!(meta.plot_id, other.plot_id);
}

#[test]
fn existing_id_is_kept() {
    let id = PlotId::new();
    let meta = PlotMeta::from_new_with_id(id, new_plot("Petak B")).unwrap();
    assert_eq!(meta.plot_id, id);
}

#[test]
fn empty_name_is_rejected() {
    assert!(PlotMeta::from_new(new_plot("   ")).is_err());
}

#[test]
fn coordinates_and_area_are_validated() {
    let mut p = new_plot("X");
    p.latitude = Some(91.0);
    assert!(PlotMeta::from_new(p).is_err());

    let mut p = new_plot("X");
    p.longitude = Some(-181.0);
    assert!(PlotMeta::from_new(p).is_err());

    let mut p = new_plot("X");
    p.area_ha = Some(0.0);
    assert!(PlotMeta::from_new(p).is_err());

    let mut p = new_plot("X");
    p.area_ha = Some(f64::NAN);
    assert!(PlotMeta::from_new(p).is_err());

    let mut p = new_plot("X");
    p.latitude = Some(-0.5);
    p.longitude = Some(117.1);
    p.area_ha = Some(1.5);
    assert!(PlotMeta::from_new(p).is_ok());
}
