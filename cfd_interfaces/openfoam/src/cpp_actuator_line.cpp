#include <array>
#include <cstddef>
#include <cstdint>
#include <new>
#include <string>
#include <type_traits>
#include <utility>

namespace rust {
inline namespace cxxbridge1 {
// #include "rust/cxx.h"

namespace {
template <typename T>
class impl;
} // namespace

class String;

#ifndef CXXBRIDGE1_RUST_STR
#define CXXBRIDGE1_RUST_STR
class Str final {
public:
  Str() noexcept;
  Str(const String &) noexcept;
  Str(const std::string &);
  Str(const char *);
  Str(const char *, std::size_t);

  Str &operator=(const Str &) & noexcept = default;

  explicit operator std::string() const;

  const char *data() const noexcept;
  std::size_t size() const noexcept;
  std::size_t length() const noexcept;
  bool empty() const noexcept;

  Str(const Str &) noexcept = default;
  ~Str() noexcept = default;

  using iterator = const char *;
  using const_iterator = const char *;
  const_iterator begin() const noexcept;
  const_iterator end() const noexcept;
  const_iterator cbegin() const noexcept;
  const_iterator cend() const noexcept;

  bool operator==(const Str &) const noexcept;
  bool operator!=(const Str &) const noexcept;
  bool operator<(const Str &) const noexcept;
  bool operator<=(const Str &) const noexcept;
  bool operator>(const Str &) const noexcept;
  bool operator>=(const Str &) const noexcept;

  void swap(Str &) noexcept;

private:
  class uninit;
  Str(uninit) noexcept;
  friend impl<Str>;

  std::array<std::uintptr_t, 2> repr;
};
#endif // CXXBRIDGE1_RUST_STR

#ifndef CXXBRIDGE1_RUST_OPAQUE
#define CXXBRIDGE1_RUST_OPAQUE
class Opaque {
public:
  Opaque() = delete;
  Opaque(const Opaque &) = delete;
  ~Opaque() = delete;
};
#endif // CXXBRIDGE1_RUST_OPAQUE

#ifndef CXXBRIDGE1_IS_COMPLETE
#define CXXBRIDGE1_IS_COMPLETE
namespace detail {
namespace {
template <typename T, typename = std::size_t>
struct is_complete : std::false_type {};
template <typename T>
struct is_complete<T, decltype(sizeof(T))> : std::true_type {};
} // namespace
} // namespace detail
#endif // CXXBRIDGE1_IS_COMPLETE

#ifndef CXXBRIDGE1_LAYOUT
#define CXXBRIDGE1_LAYOUT
class layout {
  template <typename T>
  friend std::size_t size_of();
  template <typename T>
  friend std::size_t align_of();
  template <typename T>
  static typename std::enable_if<std::is_base_of<Opaque, T>::value,
                                 std::size_t>::type
  do_size_of() {
    return T::layout::size();
  }
  template <typename T>
  static typename std::enable_if<!std::is_base_of<Opaque, T>::value,
                                 std::size_t>::type
  do_size_of() {
    return sizeof(T);
  }
  template <typename T>
  static
      typename std::enable_if<detail::is_complete<T>::value, std::size_t>::type
      size_of() {
    return do_size_of<T>();
  }
  template <typename T>
  static typename std::enable_if<std::is_base_of<Opaque, T>::value,
                                 std::size_t>::type
  do_align_of() {
    return T::layout::align();
  }
  template <typename T>
  static typename std::enable_if<!std::is_base_of<Opaque, T>::value,
                                 std::size_t>::type
  do_align_of() {
    return alignof(T);
  }
  template <typename T>
  static
      typename std::enable_if<detail::is_complete<T>::value, std::size_t>::type
      align_of() {
    return do_align_of<T>();
  }
};

template <typename T>
std::size_t size_of() {
  return layout::size_of<T>();
}

template <typename T>
std::size_t align_of() {
  return layout::align_of<T>();
}
#endif // CXXBRIDGE1_LAYOUT

namespace detail {
template <typename T, typename = void *>
struct operator_new {
  void *operator()(::std::size_t sz) { return ::operator new(sz); }
};

template <typename T>
struct operator_new<T, decltype(T::operator new(sizeof(T)))> {
  void *operator()(::std::size_t sz) { return T::operator new(sz); }
};
} // namespace detail

template <typename T>
union ManuallyDrop {
  T value;
  ManuallyDrop(T &&value) : value(::std::move(value)) {}
  ~ManuallyDrop() {}
};

template <typename T>
union MaybeUninit {
  T value;
  void *operator new(::std::size_t sz) { return detail::operator_new<T>{}(sz); }
  MaybeUninit() {}
  ~MaybeUninit() {}
};
} // namespace cxxbridge1
} // namespace rust

namespace stormbird_interface {
  struct CppActuatorLine;
}

namespace stormbird_interface {
#ifndef CXXBRIDGE1_STRUCT_stormbird_interface$CppActuatorLine
#define CXXBRIDGE1_STRUCT_stormbird_interface$CppActuatorLine
struct CppActuatorLine final : public ::rust::Opaque {
  bool use_point_sampling() const noexcept;
  double sampling_weight_limit() const noexcept;
  double projection_weight_limit() const noexcept;
  ::std::size_t nr_span_lines() const noexcept;
  ::std::size_t nr_wings() const noexcept;
  double get_local_wing_angle(::std::size_t index) const noexcept;
  void set_local_wing_angle(::std::size_t index, double angle) noexcept;
  ::std::array<double, 3> get_ctrl_point_at_index(::std::size_t index) const noexcept;
  ::std::array<double, 4> get_weighted_velocity_sampling_integral_terms_for_cell(::std::size_t line_index, ::std::array<double, 3> const &velocity, ::std::array<double, 3> const &cell_center, double cell_volume) const noexcept;
  void set_velocity_at_index(::std::size_t index, ::std::array<double, 3> velocity) noexcept;
  ::std::size_t dominating_line_element_index_at_point(::std::array<double, 3> const &point) const noexcept;
  void do_step(double time, double time_step) noexcept;
  bool update_controller(double time, double time_step) noexcept;
  ::std::array<double, 3> force_to_project(::std::size_t line_index, ::std::array<double, 3> const &velocity) const noexcept;
  double summed_projection_weights_at_point(::std::array<double, 3> const &point) const noexcept;
  void write_results(::rust::Str folder_path) const noexcept;
  ~CppActuatorLine() = delete;

private:
  friend ::rust::layout;
  struct layout {
    static ::std::size_t size() noexcept;
    static ::std::size_t align() noexcept;
  };
};
#endif // CXXBRIDGE1_STRUCT_stormbird_interface$CppActuatorLine

extern "C" {
::std::size_t stormbird_interface$cxxbridge1$CppActuatorLine$operator$sizeof() noexcept;
::std::size_t stormbird_interface$cxxbridge1$CppActuatorLine$operator$alignof() noexcept;

::stormbird_interface::CppActuatorLine *stormbird_interface$cxxbridge1$new_actuator_line_from_file(::rust::Str file_path) noexcept;

bool stormbird_interface$cxxbridge1$CppActuatorLine$use_point_sampling(::stormbird_interface::CppActuatorLine const &self) noexcept;

double stormbird_interface$cxxbridge1$CppActuatorLine$sampling_weight_limit(::stormbird_interface::CppActuatorLine const &self) noexcept;

double stormbird_interface$cxxbridge1$CppActuatorLine$projection_weight_limit(::stormbird_interface::CppActuatorLine const &self) noexcept;

::std::size_t stormbird_interface$cxxbridge1$CppActuatorLine$nr_span_lines(::stormbird_interface::CppActuatorLine const &self) noexcept;

::std::size_t stormbird_interface$cxxbridge1$CppActuatorLine$nr_wings(::stormbird_interface::CppActuatorLine const &self) noexcept;

double stormbird_interface$cxxbridge1$CppActuatorLine$get_local_wing_angle(::stormbird_interface::CppActuatorLine const &self, ::std::size_t index) noexcept;

void stormbird_interface$cxxbridge1$CppActuatorLine$set_local_wing_angle(::stormbird_interface::CppActuatorLine &self, ::std::size_t index, double angle) noexcept;

void stormbird_interface$cxxbridge1$CppActuatorLine$get_ctrl_point_at_index(::stormbird_interface::CppActuatorLine const &self, ::std::size_t index, ::std::array<double, 3> *return$) noexcept;

void stormbird_interface$cxxbridge1$CppActuatorLine$get_weighted_velocity_sampling_integral_terms_for_cell(::stormbird_interface::CppActuatorLine const &self, ::std::size_t line_index, ::std::array<double, 3> const &velocity, ::std::array<double, 3> const &cell_center, double cell_volume, ::std::array<double, 4> *return$) noexcept;

void stormbird_interface$cxxbridge1$CppActuatorLine$set_velocity_at_index(::stormbird_interface::CppActuatorLine &self, ::std::size_t index, ::std::array<double, 3> *velocity) noexcept;

::std::size_t stormbird_interface$cxxbridge1$CppActuatorLine$dominating_line_element_index_at_point(::stormbird_interface::CppActuatorLine const &self, ::std::array<double, 3> const &point) noexcept;

void stormbird_interface$cxxbridge1$CppActuatorLine$do_step(::stormbird_interface::CppActuatorLine &self, double time, double time_step) noexcept;

bool stormbird_interface$cxxbridge1$CppActuatorLine$update_controller(::stormbird_interface::CppActuatorLine &self, double time, double time_step) noexcept;

void stormbird_interface$cxxbridge1$CppActuatorLine$force_to_project(::stormbird_interface::CppActuatorLine const &self, ::std::size_t line_index, ::std::array<double, 3> const &velocity, ::std::array<double, 3> *return$) noexcept;

double stormbird_interface$cxxbridge1$CppActuatorLine$summed_projection_weights_at_point(::stormbird_interface::CppActuatorLine const &self, ::std::array<double, 3> const &point) noexcept;

void stormbird_interface$cxxbridge1$CppActuatorLine$write_results(::stormbird_interface::CppActuatorLine const &self, ::rust::Str folder_path) noexcept;
} // extern "C"

::std::size_t CppActuatorLine::layout::size() noexcept {
  return stormbird_interface$cxxbridge1$CppActuatorLine$operator$sizeof();
}

::std::size_t CppActuatorLine::layout::align() noexcept {
  return stormbird_interface$cxxbridge1$CppActuatorLine$operator$alignof();
}

::stormbird_interface::CppActuatorLine *new_actuator_line_from_file(::rust::Str file_path) noexcept {
  return stormbird_interface$cxxbridge1$new_actuator_line_from_file(file_path);
}

bool CppActuatorLine::use_point_sampling() const noexcept {
  return stormbird_interface$cxxbridge1$CppActuatorLine$use_point_sampling(*this);
}

double CppActuatorLine::sampling_weight_limit() const noexcept {
  return stormbird_interface$cxxbridge1$CppActuatorLine$sampling_weight_limit(*this);
}

double CppActuatorLine::projection_weight_limit() const noexcept {
  return stormbird_interface$cxxbridge1$CppActuatorLine$projection_weight_limit(*this);
}

::std::size_t CppActuatorLine::nr_span_lines() const noexcept {
  return stormbird_interface$cxxbridge1$CppActuatorLine$nr_span_lines(*this);
}

::std::size_t CppActuatorLine::nr_wings() const noexcept {
  return stormbird_interface$cxxbridge1$CppActuatorLine$nr_wings(*this);
}

double CppActuatorLine::get_local_wing_angle(::std::size_t index) const noexcept {
  return stormbird_interface$cxxbridge1$CppActuatorLine$get_local_wing_angle(*this, index);
}

void CppActuatorLine::set_local_wing_angle(::std::size_t index, double angle) noexcept {
  stormbird_interface$cxxbridge1$CppActuatorLine$set_local_wing_angle(*this, index, angle);
}

::std::array<double, 3> CppActuatorLine::get_ctrl_point_at_index(::std::size_t index) const noexcept {
  ::rust::MaybeUninit<::std::array<double, 3>> return$;
  stormbird_interface$cxxbridge1$CppActuatorLine$get_ctrl_point_at_index(*this, index, &return$.value);
  return ::std::move(return$.value);
}

::std::array<double, 4> CppActuatorLine::get_weighted_velocity_sampling_integral_terms_for_cell(::std::size_t line_index, ::std::array<double, 3> const &velocity, ::std::array<double, 3> const &cell_center, double cell_volume) const noexcept {
  ::rust::MaybeUninit<::std::array<double, 4>> return$;
  stormbird_interface$cxxbridge1$CppActuatorLine$get_weighted_velocity_sampling_integral_terms_for_cell(*this, line_index, velocity, cell_center, cell_volume, &return$.value);
  return ::std::move(return$.value);
}

void CppActuatorLine::set_velocity_at_index(::std::size_t index, ::std::array<double, 3> velocity) noexcept {
  ::rust::ManuallyDrop<::std::array<double, 3>> velocity$(::std::move(velocity));
  stormbird_interface$cxxbridge1$CppActuatorLine$set_velocity_at_index(*this, index, &velocity$.value);
}

::std::size_t CppActuatorLine::dominating_line_element_index_at_point(::std::array<double, 3> const &point) const noexcept {
  return stormbird_interface$cxxbridge1$CppActuatorLine$dominating_line_element_index_at_point(*this, point);
}

void CppActuatorLine::do_step(double time, double time_step) noexcept {
  stormbird_interface$cxxbridge1$CppActuatorLine$do_step(*this, time, time_step);
}

bool CppActuatorLine::update_controller(double time, double time_step) noexcept {
  return stormbird_interface$cxxbridge1$CppActuatorLine$update_controller(*this, time, time_step);
}

::std::array<double, 3> CppActuatorLine::force_to_project(::std::size_t line_index, ::std::array<double, 3> const &velocity) const noexcept {
  ::rust::MaybeUninit<::std::array<double, 3>> return$;
  stormbird_interface$cxxbridge1$CppActuatorLine$force_to_project(*this, line_index, velocity, &return$.value);
  return ::std::move(return$.value);
}

double CppActuatorLine::summed_projection_weights_at_point(::std::array<double, 3> const &point) const noexcept {
  return stormbird_interface$cxxbridge1$CppActuatorLine$summed_projection_weights_at_point(*this, point);
}

void CppActuatorLine::write_results(::rust::Str folder_path) const noexcept {
  stormbird_interface$cxxbridge1$CppActuatorLine$write_results(*this, folder_path);
}
} // namespace stormbird_interface
